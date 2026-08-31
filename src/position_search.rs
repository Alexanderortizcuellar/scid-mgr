use anyhow::{Context, Result};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, Color, EnPassantMode, Move, Position, Role, Square};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const ENCODE_END_GAME: u8 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMatch {
    pub game_id: usize,
    pub ply: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSearchResult {
    pub target_fen: String,
    pub target_hash: u64,
    pub matches: Vec<PositionMatch>,
    pub total_games_searched: usize,
    pub elapsed_ms: f64,
}

/// Skips extra tags in the game blob according to SCID specification
#[inline]
pub fn skip_extra_tags(blob: &[u8], cursor: &mut usize) -> bool {
    while *cursor < blob.len() {
        let name_code = blob[*cursor];
        *cursor += 1;
        if name_code == 0 {
            return true;
        }
        if name_code == 255 {
            *cursor += 3;
            continue;
        }
        if name_code <= 240 {
            *cursor += name_code as usize;
        }
        // 241..=254 has no name to skip
        if *cursor >= blob.len() {
            return false;
        }
        let value_len = blob[*cursor] as usize;
        *cursor += 1 + value_len;
    }
    false
}

/// Parses the initial position and advances cursor past flags and optional FEN
#[inline]
pub fn parse_start_position(blob: &[u8], cursor: &mut usize) -> Option<Chess> {
    if !skip_extra_tags(blob, cursor) || *cursor >= blob.len() {
        return None;
    }

    let flags = blob[*cursor];
    *cursor += 1;

    if flags & 0x01 != 0 {
        let fen_start = *cursor;
        while *cursor < blob.len() && blob[*cursor] != 0 {
            *cursor += 1;
        }
        let fen_bytes = &blob[fen_start..*cursor];
        if *cursor < blob.len() {
            *cursor += 1; // consume null byte
        }
        let fen_str = std::str::from_utf8(fen_bytes).ok()?;
        let fen: shakmaty::fen::Fen = fen_str.parse().ok()?;
        fen.into_position(shakmaty::CastlingMode::Standard).ok()
    } else {
        Some(Chess::default())
    }
}

/// Standard starting piece table mapping (16 slots per side)
#[inline]
pub(crate) fn standard_piece_slots() -> [[u8; 16]; 2] {
    [
        // White: 0:K(e1=4), 1:Ra1(0), 2:Nb1(1), 3:Bc1(2), 4:Qd1(3), 5:Bf1(5), 6:Ng1(6), 7:Rh1(7), 8..15: Pawns a2..h2 (8..15)
        [4, 0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        // Black: 0:K(e8=60), 1:Ra8(56), 2:Nb8(57), 3:Bc8(58), 4:Qd8(59), 5:Bf8(61), 6:Ng8(62), 7:Rh8(63), 8..15: Pawns a7..h7 (48..55)
        [60, 56, 57, 58, 59, 61, 62, 63, 48, 49, 50, 51, 52, 53, 54, 55],
    ]
}

#[inline]
pub(crate) fn update_slots_on_move(
    slots: &mut [[u8; 16]; 2],
    counts: &mut [usize; 2],
    side_idx: usize,
    piece_idx: usize,
    to_sq: u8,
    is_castle_kingside: bool,
    is_castle_queenside: bool,
    captured_sq: Option<u8>,
) {
    slots[side_idx][piece_idx] = to_sq;

    if is_castle_kingside || is_castle_queenside {
        let (rook_from, rook_to) = if side_idx == 0 {
            if is_castle_kingside { (7, 5) } else { (0, 3) }
        } else {
            if is_castle_kingside { (63, 61) } else { (56, 59) }
        };
        if let Some(r_idx) = (0..counts[side_idx]).find(|&i| slots[side_idx][i] == rook_from) {
            slots[side_idx][r_idx] = rook_to;
        }
    }

    if let Some(cap_sq) = captured_sq {
        let enemy_idx = 1 - side_idx;
        if let Some(cap_idx) = (0..counts[enemy_idx]).find(|&i| slots[enemy_idx][i] == cap_sq) {
            counts[enemy_idx] -= 1;
            slots[enemy_idx][cap_idx] = slots[enemy_idx][counts[enemy_idx]];
        }
    }
}

/// Decodes next move directly from raw byte stream
pub(crate) fn decode_raw_move(
    byte: u8,
    cursor: &mut usize,
    blob: &[u8],
    pos: &Chess,
    slots: &[[u8; 16]; 2],
    counts: &[usize; 2],
) -> Option<(Move, usize, u8, bool, bool, Option<u8>)> {
    let piece_idx = (byte >> 4) as usize;
    let code = (byte & 0x0F) as i32;

    let color = pos.turn();
    let side_idx = usize::from(color == Color::Black);

    if piece_idx >= counts[side_idx] {
        return None;
    }

    let from_u8 = slots[side_idx][piece_idx];
    let from_sq = Square::try_from(from_u8).ok()?;
    let piece = pos.board().piece_at(from_sq)?;

    let from_idx = i32::from(from_u8);

    let (to_sq, promo, is_castle_k, is_castle_q) = match piece.role {
        Role::Pawn => {
            const PROMO: [Option<Role>; 16] = [
                None, None, None,
                Some(Role::Queen), Some(Role::Queen), Some(Role::Queen),
                Some(Role::Rook), Some(Role::Rook), Some(Role::Rook),
                Some(Role::Bishop), Some(Role::Bishop), Some(Role::Bishop),
                Some(Role::Knight), Some(Role::Knight), Some(Role::Knight),
                None,
            ];
            const SQDIFF: [i32; 16] = [7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9, 16];
            let idx = code as usize;
            if idx >= 16 { return None; }
            let diff = SQDIFF[idx];
            let to = if color == Color::White { from_idx + diff } else { from_idx - diff };
            if !(0..64).contains(&to) { return None; }
            (Square::try_from(to as u8).ok()?, PROMO[idx], false, false)
        }
        Role::Knight => {
            const SQDIFF: [i32; 16] = [0, -17, -15, -10, -6, 6, 10, 15, 17, 0, 0, 0, 0, 0, 0, 0];
            let idx = code as usize;
            if idx >= 16 { return None; }
            let to = from_idx + SQDIFF[idx];
            if !(0..64).contains(&to) { return None; }
            (Square::try_from(to as u8).ok()?, None, false, false)
        }
        Role::Bishop => {
            let fylediff = (code & 0x07) - i32::from(from_sq.file() as u8);
            let to = if code >= 8 { from_idx - 7 * fylediff } else { from_idx + 9 * fylediff };
            if !(0..64).contains(&to) { return None; }
            (Square::try_from(to as u8).ok()?, None, false, false)
        }
        Role::Rook => {
            let to = if code < 8 {
                i32::from(from_sq.rank() as u8) * 8 + code
            } else {
                (code - 8) * 8 + i32::from(from_sq.file() as u8)
            };
            if !(0..64).contains(&to) { return None; }
            (Square::try_from(to as u8).ok()?, None, false, false)
        }
        Role::Queen => {
            if code == i32::from(from_sq.file() as u8) {
                if *cursor >= blob.len() { return None; }
                let b2 = blob[*cursor];
                *cursor += 1;
                let to = i32::from(b2) - 64;
                if !(0..64).contains(&to) { return None; }
                (Square::try_from(to as u8).ok()?, None, false, false)
            } else {
                let to = if code < 8 {
                    i32::from(from_sq.rank() as u8) * 8 + code
                } else {
                    (code - 8) * 8 + i32::from(from_sq.file() as u8)
                };
                if !(0..64).contains(&to) { return None; }
                (Square::try_from(to as u8).ok()?, None, false, false)
            }
        }
        Role::King => {
            if code == 0 { return None; } // null move
            if code <= 8 {
                const SQDIFF: [i32; 9] = [0, -9, -8, -7, -1, 1, 7, 8, 9];
                let to = from_idx + SQDIFF[code as usize];
                if !(0..64).contains(&to) { return None; }
                (Square::try_from(to as u8).ok()?, None, false, false)
            } else if code == 9 {
                let to_sq = if color == Color::White { Square::C1 } else { Square::C8 };
                (to_sq, None, false, true)
            } else if code == 10 {
                let to_sq = if color == Color::White { Square::G1 } else { Square::G8 };
                (to_sq, None, true, false)
            } else {
                return None;
            }
        }
    };

    let mv = if is_castle_k || is_castle_q {
        let (king, rook) = if color == Color::White {
            if is_castle_k { (Square::E1, Square::H1) } else { (Square::E1, Square::A1) }
        } else {
            if is_castle_k { (Square::E8, Square::H8) } else { (Square::E8, Square::A8) }
        };
        Move::Castle { king, rook }
    } else if piece.role == Role::Pawn && pos.board().piece_at(to_sq).is_none() && from_sq.file() != to_sq.file() {
        Move::EnPassant { from: from_sq, to: to_sq }
    } else {
        let capture = pos.board().piece_at(to_sq).map(|p| p.role);
        Move::Normal {
            role: piece.role,
            from: from_sq,
            to: to_sq,
            capture,
            promotion: promo,
        }
    };

    let captured_sq = if let Move::EnPassant { from, to } = mv {
        Some(u8::from(Square::from_coords(to.file(), from.rank())))
    } else if pos.board().piece_at(to_sq).is_some() {
        Some(u8::from(to_sq))
    } else {
        None
    };

    Some((mv, piece_idx, u8::from(to_sq), is_castle_k, is_castle_q, captured_sq))
}

/// Search for a target position across all games directly in .sg5 / .sg4 via memory mapping with streaming progress
pub fn search_position_mmap_with_progress<F>(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    target_pos: &Chess,
    max_ply: Option<usize>,
    progress: F,
) -> Result<PositionSearchResult>
where
    F: Fn(usize, usize, usize) + Sync,
{
    let start_time = Instant::now();
    let file = File::open(games_path)
        .with_context(|| format!("Failed to open games file: {}", games_path.display()))?;
    let mmap = unsafe { Mmap::map(&file)? };

    let target_hash: Zobrist64 = target_pos.zobrist_hash(EnPassantMode::Legal);
    let target_hash_u64 = target_hash.0;
    let max_search_ply = max_ply.unwrap_or(250);

    let initial_hash: Zobrist64 = Chess::default().zobrist_hash(EnPassantMode::Legal);
    let is_initial_pos = target_hash == initial_hash;

    let total = entries.len();
    let chunk_size = 50_000.max(total / 100).max(1);
    let scanned_counter = AtomicUsize::new(0);
    let match_counter = AtomicUsize::new(0);

    let matches: Vec<PositionMatch> = entries
        .par_chunks(chunk_size)
        .enumerate()
        .flat_map(|(chunk_idx, chunk)| {
            let chunk_start_id = chunk_idx * chunk_size;
            let chunk_matches: Vec<PositionMatch> = chunk
                .iter()
                .enumerate()
                .filter_map(|(local_id, entry)| {
                    let game_id = chunk_start_id + local_id;
                    if entry.deleted {
                        return None;
                    }

                    if is_initial_pos {
                        return Some(PositionMatch { game_id, ply: 0 });
                    }

                    let start = entry.offset as usize;
                    let end = start + entry.length as usize;
                    if end > mmap.len() || start >= end {
                        return None;
                    }

                    let blob = &mmap[start..end];
                    if blob.len() < 2 {
                        return None;
                    }

                    let mut cursor = 0;
                    let mut pos = parse_start_position(blob, &mut cursor)?;

                    let mut slots = standard_piece_slots();
                    let mut counts = [16usize, 16];
                    let mut ply = 0;

                    // Step through move stream
                    while cursor < blob.len() && ply <= max_search_ply {
                        let byte = blob[cursor];
                        cursor += 1;

                        if byte == ENCODE_END_GAME {
                            break;
                        }
                        if byte == 11 {
                            // NAG
                            cursor += 1;
                            continue;
                        }
                        if byte == 12 {
                            // Comment marker
                            continue;
                        }
                        if byte == 13 || byte == 14 {
                            // Variation markers - skip or follow mainline
                            continue;
                        }

                        let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) = match decode_raw_move(
                            byte,
                            &mut cursor,
                            blob,
                            &pos,
                            &slots,
                            &counts,
                        ) {
                            Some(m) => m,
                            None => break,
                        };

                        let side_idx = usize::from(pos.turn() == Color::Black);
                        update_slots_on_move(
                            &mut slots,
                            &mut counts,
                            side_idx,
                            piece_idx,
                            to_sq,
                            is_castle_k,
                            is_castle_q,
                            captured_sq,
                        );

                        if !pos.is_legal(&mv) {
                            break;
                        }
                        pos.play_unchecked(&mv);
                        ply += 1;

                        let h: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                        if h.0 == target_hash_u64 {
                            return Some(PositionMatch { game_id, ply });
                        }
                    }

                    None
                })
                .collect();

            let cur_m = match_counter.fetch_add(chunk_matches.len(), Ordering::Relaxed) + chunk_matches.len();
            let cur_s = scanned_counter.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
            progress(cur_s.min(total), total, cur_m);

            chunk_matches
        })
        .collect();

    let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    let target_fen = format!("{:?}", target_pos);

    Ok(PositionSearchResult {
        target_fen,
        target_hash: target_hash_u64,
        matches,
        total_games_searched: entries.len(),
        elapsed_ms,
    })
}

/// Search for a target position across all games directly in .sg5 / .sg4 via memory mapping
pub fn search_position_mmap(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    target_pos: &Chess,
    max_ply: Option<usize>,
) -> Result<PositionSearchResult> {
    search_position_mmap_with_progress(entries, games_path, target_pos, max_ply, |_, _, _| {})
}

/// Parses FEN or partial board string into list of required (Square, Role, Color) pieces
pub fn parse_piece_placements(board_str: &str) -> Vec<(Square, Role, Color)> {
    let mut pieces = Vec::new();
    let board_part = board_str.trim().split_whitespace().next().unwrap_or(board_str.trim());
    let ranks: Vec<&str> = board_part.split('/').collect();
    if ranks.len() != 8 {
        return pieces;
    }

    for (rank_idx, rank_str) in ranks.iter().enumerate() {
        let rank = 7 - rank_idx as u8;
        let mut file = 0u8;
        for ch in rank_str.chars() {
            if let Some(digit) = ch.to_digit(10) {
                file += digit as u8;
            } else {
                let color = if ch.is_uppercase() {
                    Color::White
                } else {
                    Color::Black
                };
                let role = match ch.to_ascii_lowercase() {
                    'p' => Some(Role::Pawn),
                    'n' => Some(Role::Knight),
                    'b' => Some(Role::Bishop),
                    'r' => Some(Role::Rook),
                    'q' => Some(Role::Queen),
                    'k' => Some(Role::King),
                    _ => None,
                };
                if let Some(r) = role {
                    if file < 8 {
                        if let Ok(sq) = Square::try_from(rank * 8 + file) {
                            pieces.push((sq, r, color));
                        }
                    }
                }
                file += 1;
            }
        }
    }

    pieces
}

#[inline]
pub fn matches_piece_placements(pos: &Chess, required: &[(Square, Role, Color)]) -> bool {
    let board = pos.board();
    for &(sq, role, color) in required {
        match board.piece_at(sq) {
            Some(p) if p.role == role && p.color == color => {}
            _ => return false,
        }
    }
    true
}

/// Search for games matching specific piece placements (e.g. Queen on d4) across all games with streaming progress
pub fn search_piece_placements_mmap_with_progress<F>(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    required: &[(Square, Role, Color)],
    match_any_ply: bool,
    max_ply: Option<usize>,
    progress: F,
) -> Result<Vec<usize>>
where
    F: Fn(usize, usize, usize) + Sync,
{
    if required.is_empty() {
        return Ok(Vec::new());
    }

    let file = File::open(games_path)
        .with_context(|| format!("Failed to open games file: {}", games_path.display()))?;
    let mmap = unsafe { Mmap::map(&file)? };

    let max_search_ply = max_ply.unwrap_or(250);
    let total = entries.len();
    let chunk_size = 50_000.max(total / 100).max(1);
    let scanned_counter = AtomicUsize::new(0);
    let match_counter = AtomicUsize::new(0);

    let matches: Vec<usize> = entries
        .par_chunks(chunk_size)
        .enumerate()
        .flat_map(|(chunk_idx, chunk)| {
            let chunk_start_id = chunk_idx * chunk_size;
            let chunk_matches: Vec<usize> = chunk
                .iter()
                .enumerate()
                .filter_map(|(local_id, entry)| {
                    let game_id = chunk_start_id + local_id;
                    if entry.deleted {
                        return None;
                    }

                    let start = entry.offset as usize;
                    let end = start + entry.length as usize;
                    if end > mmap.len() || start >= end {
                        return None;
                    }

                    let blob = &mmap[start..end];
                    if blob.len() < 2 {
                        return None;
                    }

                    let mut cursor = 0;
                    let mut pos = parse_start_position(blob, &mut cursor)?;

                    let mut slots = standard_piece_slots();
                    let mut counts = [16usize, 16];
                    let mut ply = 0;

                    if match_any_ply && matches_piece_placements(&pos, required) {
                        return Some(game_id);
                    }

                    while cursor < blob.len() && ply <= max_search_ply {
                        let byte = blob[cursor];
                        cursor += 1;

                        if byte == ENCODE_END_GAME {
                            break;
                        }
                        if byte == 11 {
                            cursor += 1;
                            continue;
                        }
                        if byte == 12 || byte == 13 || byte == 14 {
                            continue;
                        }

                        let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) = match decode_raw_move(
                            byte,
                            &mut cursor,
                            blob,
                            &pos,
                            &slots,
                            &counts,
                        ) {
                            Some(m) => m,
                            None => break,
                        };

                        let side_idx = usize::from(pos.turn() == Color::Black);
                        update_slots_on_move(
                            &mut slots,
                            &mut counts,
                            side_idx,
                            piece_idx,
                            to_sq,
                            is_castle_k,
                            is_castle_q,
                            captured_sq,
                        );

                        if !pos.is_legal(&mv) {
                            break;
                        }
                        pos.play_unchecked(&mv);
                        ply += 1;

                        if match_any_ply && matches_piece_placements(&pos, required) {
                            return Some(game_id);
                        }
                    }

                    if !match_any_ply && matches_piece_placements(&pos, required) {
                        Some(game_id)
                    } else {
                        None
                    }
                })
                .collect();

            let cur_m = match_counter.fetch_add(chunk_matches.len(), Ordering::Relaxed) + chunk_matches.len();
            let cur_s = scanned_counter.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
            progress(cur_s.min(total), total, cur_m);

            chunk_matches
        })
        .collect();

    Ok(matches)
}

/// Search for games matching specific piece placements (e.g. Queen on d4) across all games
pub fn search_piece_placements_mmap(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    required: &[(Square, Role, Color)],
    match_any_ply: bool,
    max_ply: Option<usize>,
) -> Result<Vec<usize>> {
    search_piece_placements_mmap_with_progress(entries, games_path, required, match_any_ply, max_ply, |_, _, _| {})
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialFilter {
    pub white_queens: Option<u8>,
    pub white_rooks: Option<u8>,
    pub white_bishops: Option<u8>,
    pub white_knights: Option<u8>,
    pub white_pawns: Option<u8>,

    pub black_queens: Option<u8>,
    pub black_rooks: Option<u8>,
    pub black_bishops: Option<u8>,
    pub black_knights: Option<u8>,
    pub black_pawns: Option<u8>,

    pub opposite_bishops: Option<bool>,
    pub same_bishops: Option<bool>,

    pub match_any_ply: bool,
    pub max_ply: Option<usize>,
}

#[inline]
pub fn matches_material(pos: &Chess, filter: &MaterialFilter) -> bool {
    let board = pos.board();
    let white = board.white();
    let black = board.black();

    // Exact White piece counts
    if let Some(q) = filter.white_queens {
        if (board.queens() & white).count() as u8 != q {
            return false;
        }
    }
    if let Some(r) = filter.white_rooks {
        if (board.rooks() & white).count() as u8 != r {
            return false;
        }
    }
    if let Some(b) = filter.white_bishops {
        if (board.bishops() & white).count() as u8 != b {
            return false;
        }
    }
    if let Some(n) = filter.white_knights {
        if (board.knights() & white).count() as u8 != n {
            return false;
        }
    }
    if let Some(p) = filter.white_pawns {
        if (board.pawns() & white).count() as u8 != p {
            return false;
        }
    }

    // Exact Black piece counts
    if let Some(q) = filter.black_queens {
        if (board.queens() & black).count() as u8 != q {
            return false;
        }
    }
    if let Some(r) = filter.black_rooks {
        if (board.rooks() & black).count() as u8 != r {
            return false;
        }
    }
    if let Some(b) = filter.black_bishops {
        if (board.bishops() & black).count() as u8 != b {
            return false;
        }
    }
    if let Some(n) = filter.black_knights {
        if (board.knights() & black).count() as u8 != n {
            return false;
        }
    }
    if let Some(p) = filter.black_pawns {
        if (board.pawns() & black).count() as u8 != p {
            return false;
        }
    }

    // Opposite / Same colored bishops verification
    if filter.opposite_bishops.unwrap_or(false) || filter.same_bishops.unwrap_or(false) {
        let w_bishops = board.bishops() & white;
        let b_bishops = board.bishops() & black;
        if w_bishops.is_empty() || b_bishops.is_empty() {
            return false;
        }

        let w_light = !(w_bishops & shakmaty::Bitboard::LIGHT_SQUARES).is_empty();
        let b_light = !(b_bishops & shakmaty::Bitboard::LIGHT_SQUARES).is_empty();
        let is_opposite = w_light != b_light;

        if let Some(req_opp) = filter.opposite_bishops {
            if req_opp && !is_opposite {
                return false;
            }
        }
        if let Some(req_same) = filter.same_bishops {
            if req_same && is_opposite {
                return false;
            }
        }
    }

    true
}

/// Search for games matching specific piece material counts across all games with streaming progress
pub fn search_material_mmap_with_progress<F>(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    filter: &MaterialFilter,
    progress: F,
) -> Result<Vec<usize>>
where
    F: Fn(usize, usize, usize) + Sync,
{
    let file = File::open(games_path)
        .with_context(|| format!("Failed to open games file: {}", games_path.display()))?;
    let mmap = unsafe { Mmap::map(&file)? };

    let max_search_ply = filter.max_ply.unwrap_or(250);
    let match_any_ply = filter.match_any_ply;

    let total = entries.len();
    let chunk_size = 50_000.max(total / 100).max(1);
    let scanned_counter = AtomicUsize::new(0);
    let match_counter = AtomicUsize::new(0);

    let matches: Vec<usize> = entries
        .par_chunks(chunk_size)
        .enumerate()
        .flat_map(|(chunk_idx, chunk)| {
            let chunk_start_id = chunk_idx * chunk_size;
            let chunk_matches: Vec<usize> = chunk
                .iter()
                .enumerate()
                .filter_map(|(local_id, entry)| {
                    let game_id = chunk_start_id + local_id;
                    if entry.deleted {
                        return None;
                    }

                    let start = entry.offset as usize;
                    let end = start + entry.length as usize;
                    if end > mmap.len() || start >= end {
                        return None;
                    }

                    let blob = &mmap[start..end];
                    if blob.len() < 2 {
                        return None;
                    }

                    let mut cursor = 0;
                    let mut pos = parse_start_position(blob, &mut cursor)?;

                    let mut slots = standard_piece_slots();
                    let mut counts = [16usize, 16];
                    let mut ply = 0;

                    if match_any_ply && matches_material(&pos, filter) {
                        return Some(game_id);
                    }

                    while cursor < blob.len() && ply <= max_search_ply {
                        let byte = blob[cursor];
                        cursor += 1;

                        if byte == ENCODE_END_GAME {
                            break;
                        }
                        if byte == 11 {
                            cursor += 1;
                            continue;
                        }
                        if byte == 12 || byte == 13 || byte == 14 {
                            continue;
                        }

                        let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) = match decode_raw_move(
                            byte,
                            &mut cursor,
                            blob,
                            &pos,
                            &slots,
                            &counts,
                        ) {
                            Some(m) => m,
                            None => break,
                        };

                        let side_idx = usize::from(pos.turn() == Color::Black);
                        update_slots_on_move(
                            &mut slots,
                            &mut counts,
                            side_idx,
                            piece_idx,
                            to_sq,
                            is_castle_k,
                            is_castle_q,
                            captured_sq,
                        );

                        if !pos.is_legal(&mv) {
                            break;
                        }
                        pos.play_unchecked(&mv);
                        ply += 1;

                        if match_any_ply && matches_material(&pos, filter) {
                            return Some(game_id);
                        }
                    }

                    if !match_any_ply && matches_material(&pos, filter) {
                        Some(game_id)
                    } else {
                        None
                    }
                })
                .collect();

            let cur_m = match_counter.fetch_add(chunk_matches.len(), Ordering::Relaxed) + chunk_matches.len();
            let cur_s = scanned_counter.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
            progress(cur_s.min(total), total, cur_m);

            chunk_matches
        })
        .collect();

    Ok(matches)
}

/// Search for games matching specific piece material counts across all games
pub fn search_material_mmap(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    filter: &MaterialFilter,
) -> Result<Vec<usize>> {
    search_material_mmap_with_progress(entries, games_path, filter, |_, _, _| {})
}
