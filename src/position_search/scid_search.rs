use anyhow::{Context, Result};
use memmap2::Mmap;
use rayon::prelude::*;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, Color, EnPassantMode, Position, Role, Square};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::decoder::{
    decode_raw_move, parse_start_position, standard_piece_slots, update_slots_on_move,
    ENCODE_END_GAME,
};
use super::parser::matches_piece_placements;
use super::types::{MaterialFilter, PositionMatch, PositionSearchResult, PositionTargetMatcher};

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

/// Search for a target position across all games directly in .sg5 / .sg4 via memory mapping with streaming progress
pub fn search_position_matcher_mmap_with_progress<F>(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: &Path,
    matcher: &PositionTargetMatcher,
    max_ply: Option<usize>,
    progress: F,
) -> Result<Vec<PositionMatch>>
where
    F: Fn(usize, usize, usize) + Sync,
{
    let file = File::open(games_path)
        .with_context(|| format!("Failed to open games file: {}", games_path.display()))?;
    let mmap = unsafe { Mmap::map(&file)? };

    let max_search_ply = max_ply.unwrap_or(250);
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

                    // Check initial position (ply 0)
                    if matcher.matches(&pos) {
                        return Some(PositionMatch { game_id, ply: 0 });
                    }

                    let mut slots = standard_piece_slots();
                    let mut counts = [16usize, 16];
                    let mut ply = 0;

                    // Step through move stream
                    while cursor < blob.len() && ply < max_search_ply {
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

                        let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) =
                            match decode_raw_move(byte, &mut cursor, blob, &pos, &slots, &counts) {
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

                        if matcher.matches(&pos) {
                            return Some(PositionMatch { game_id, ply });
                        }
                    }

                    None
                })
                .collect();

            let cur_m = match_counter.fetch_add(chunk_matches.len(), Ordering::Relaxed)
                + chunk_matches.len();
            let cur_s = scanned_counter.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
            progress(cur_s.min(total), total, cur_m);

            chunk_matches
        })
        .collect();

    Ok(matches)
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
    let matcher = PositionTargetMatcher::BoardWithTurn {
        board: target_pos.board().clone(),
        turn: Some(target_pos.turn()),
    };
    let matches = search_position_matcher_mmap_with_progress(
        entries, games_path, &matcher, max_ply, progress,
    )?;
    let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    let h: Zobrist64 = target_pos.zobrist_hash(EnPassantMode::Legal);
    let target_hash_u64 = h.0;
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

                        let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) =
                            match decode_raw_move(byte, &mut cursor, blob, &pos, &slots, &counts) {
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

            let cur_m = match_counter.fetch_add(chunk_matches.len(), Ordering::Relaxed)
                + chunk_matches.len();
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
    search_piece_placements_mmap_with_progress(
        entries,
        games_path,
        required,
        match_any_ply,
        max_ply,
        |_, _, _| {},
    )
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

                        let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) =
                            match decode_raw_move(byte, &mut cursor, blob, &pos, &slots, &counts) {
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

            let cur_m = match_counter.fetch_add(chunk_matches.len(), Ordering::Relaxed)
                + chunk_matches.len();
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
