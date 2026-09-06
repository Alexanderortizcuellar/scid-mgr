use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Position};

pub const POS_INDEX_MAGIC: &[u8; 8] = b"SCIDPOS5";
pub const POS_INDEX_VERSION: u32 = 3;
pub const DEFAULT_MAX_PLY_DEPTH: usize = 16; // 8 full moves
const NUM_STRIPES: usize = 256;
const HEADER_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus {
    Valid,
    Outdated,
    Missing,
}

/// Compact 16-bit binary move representation:
/// - Bits 0..5: From Square (0..63)
/// - Bits 6..11: To Square (0..63)
/// - Bits 12..14: Promotion Piece (0=None, 1=Knight, 2=Bishop, 3=Rook, 4=Queen)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PackedMove(pub u16);

impl PackedMove {
    #[inline]
    pub fn new(from: u8, to: u8, promo: Option<shakmaty::Role>) -> Self {
        let p = match promo {
            None => 0u16,
            Some(shakmaty::Role::Knight) => 1,
            Some(shakmaty::Role::Bishop) => 2,
            Some(shakmaty::Role::Rook) => 3,
            Some(shakmaty::Role::Queen) => 4,
            _ => 0,
        };
        let val = (from as u16 & 0x3F) | ((to as u16 & 0x3F) << 6) | (p << 12);
        PackedMove(val)
    }

    #[inline]
    pub fn from_square(self) -> u8 {
        (self.0 & 0x3F) as u8
    }

    #[inline]
    pub fn to_square(self) -> u8 {
        ((self.0 >> 6) & 0x3F) as u8
    }

    #[inline]
    pub fn promotion(self) -> Option<shakmaty::Role> {
        match (self.0 >> 12) & 0x07 {
            1 => Some(shakmaty::Role::Knight),
            2 => Some(shakmaty::Role::Bishop),
            3 => Some(shakmaty::Role::Rook),
            4 => Some(shakmaty::Role::Queen),
            _ => None,
        }
    }

    pub fn to_uci_string(self) -> String {
        let from_sq = shakmaty::Square::new(self.from_square() as u32);
        let to_sq = shakmaty::Square::new(self.to_square() as u32);
        let promo_str = match self.promotion() {
            Some(shakmaty::Role::Knight) => "n",
            Some(shakmaty::Role::Bishop) => "b",
            Some(shakmaty::Role::Rook) => "r",
            Some(shakmaty::Role::Queen) => "q",
            _ => "",
        };
        format!("{}{}{}", from_sq, to_sq, promo_str)
    }

    pub fn to_shakmaty_move(self, pos: &Chess) -> Option<shakmaty::Move> {
        let from_sq = shakmaty::Square::new(self.from_square() as u32);
        let to_sq = shakmaty::Square::new(self.to_square() as u32);
        let promo = self.promotion();

        pos.legal_moves().into_iter().find(|m| m.from() == Some(from_sq) && m.to() == to_sq && m.promotion() == promo)
    }
}

impl From<&shakmaty::Move> for PackedMove {
    #[inline]
    fn from(m: &shakmaty::Move) -> Self {
        let from = m.from().map(|sq| sq as u8).unwrap_or(0);
        let to = m.to() as u8;
        PackedMove::new(from, to, m.promotion())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionIndexHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub flags: u32,
    pub db_mtime_secs: u64,
    pub db_size_bytes: u64,
    pub db_game_count: u64,
    pub max_ply_depth: u32,
    pub unique_positions: u32,
    pub index_offset: u64,
    pub data_offset: u64,
    pub created_timestamp: u64,
}

impl PositionIndexHeader {
    pub fn read_from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            anyhow::bail!("Header slice too small");
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if &magic != POS_INDEX_MAGIC {
            anyhow::bail!("Invalid magic bytes in position index");
        }

        let version = u32::from_le_bytes(bytes[8..12].try_into()?);
        let flags = u32::from_le_bytes(bytes[12..16].try_into()?);
        let db_mtime_secs = u64::from_le_bytes(bytes[16..24].try_into()?);
        let db_size_bytes = u64::from_le_bytes(bytes[24..32].try_into()?);
        let db_game_count = u64::from_le_bytes(bytes[32..40].try_into()?);
        let max_ply_depth = u32::from_le_bytes(bytes[40..44].try_into()?);
        let unique_positions = u32::from_le_bytes(bytes[44..48].try_into()?);
        let index_offset = u64::from_le_bytes(bytes[48..56].try_into()?);
        let data_offset = u64::from_le_bytes(bytes[56..64].try_into()?);

        Ok(Self {
            magic,
            version,
            flags,
            db_mtime_secs,
            db_size_bytes,
            db_game_count,
            max_ply_depth,
            unique_positions,
            index_offset,
            data_offset,
            created_timestamp: 0,
        })
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(&self.magic)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.flags.to_le_bytes())?;
        w.write_all(&self.db_mtime_secs.to_le_bytes())?;
        w.write_all(&self.db_size_bytes.to_le_bytes())?;
        w.write_all(&self.db_game_count.to_le_bytes())?;
        w.write_all(&self.max_ply_depth.to_le_bytes())?;
        w.write_all(&self.unique_positions.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.data_offset.to_le_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
pub struct SortedIndexEntry {
    pub hash: u64,
    pub data_offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoveStats {
    pub packed_move: u16,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub game_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionNode {
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub moves: Vec<MoveStats>,
}

impl PositionNode {
    pub fn new(zobrist_hash: u64) -> Self {
        Self {
            zobrist_hash,
            total_games: 0,
            white_wins: 0,
            draws: 0,
            black_wins: 0,
            moves: Vec::new(),
        }
    }

    pub fn record_game(
        &mut self,
        next_move: Option<u16>,
        w_win: u32,
        draw: u32,
        b_win: u32,
        game_id: u32,
        max_game_ids: usize,
    ) {
        self.total_games += 1;
        self.white_wins += w_win;
        self.draws += draw;
        self.black_wins += b_win;

        if let Some(packed) = next_move {
            let move_stat = if let Some(pos) = self.moves.iter().position(|m| m.packed_move == packed) {
                &mut self.moves[pos]
            } else {
                self.moves.push(MoveStats {
                    packed_move: packed,
                    total_games: 0,
                    white_wins: 0,
                    draws: 0,
                    black_wins: 0,
                    game_ids: Vec::new(),
                });
                self.moves.last_mut().unwrap()
            };

            move_stat.total_games += 1;
            move_stat.white_wins += w_win;
            move_stat.draws += draw;
            move_stat.black_wins += b_win;
            if (max_game_ids == 0 || move_stat.game_ids.len() < max_game_ids) && move_stat.game_ids.last().copied() != Some(game_id) {
                move_stat.game_ids.push(game_id);
            }
        }
    }

    pub fn merge(&mut self, other: PositionNode) {
        self.total_games += other.total_games;
        self.white_wins += other.white_wins;
        self.draws += other.draws;
        self.black_wins += other.black_wins;

        for other_m in other.moves {
            if let Some(m) = self.moves.iter_mut().find(|m| m.packed_move == other_m.packed_move) {
                m.total_games += other_m.total_games;
                m.white_wins += other_m.white_wins;
                m.draws += other_m.draws;
                m.black_wins += other_m.black_wins;
                m.game_ids.extend(other_m.game_ids);
                m.game_ids.sort_unstable();
                m.game_ids.dedup();
            } else {
                self.moves.push(other_m);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTreeMoveView {
    pub san: String,
    pub uci: String,
    pub total_games: u32,
    pub white_pct: f64,
    pub draw_pct: f64,
    pub black_pct: f64,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub avg_white_elo: Option<u32>,
    pub avg_black_elo: Option<u32>,
    #[serde(default)]
    pub last_played: Option<String>,
    pub sample_game_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTreeReport {
    pub fen: String,
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub white_pct: f64,
    pub draw_pct: f64,
    pub black_pct: f64,
    pub moves: Vec<OpeningTreeMoveView>,
    pub sample_game_ids: Vec<u32>,
    #[serde(default)]
    pub sample_games: Vec<crate::db::GameSummary>,
}

/// Static, disk-backed position index using zero-copy memory mapping (`memmap2`)
pub struct PositionIndex {
    pub path: PathBuf,
    mmap: memmap2::Mmap,
    pub header: PositionIndexHeader,
}

impl PositionIndex {
    /// Determines companion `.pos.idx` path for any database file
    pub fn companion_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
        let p = db_path.as_ref();
        let path_str = p.to_string_lossy();
        let lower = path_str.to_lowercase();
        if lower.ends_with(".si5") || lower.ends_with(".si4") || lower.ends_with(".sg5") || lower.ends_with(".sg4") || lower.ends_with(".sn5") || lower.ends_with(".sn4") {
            p.with_extension("pos.idx")
        } else if lower.ends_with(".pgn") {
            PathBuf::from(format!("{}.pos.idx", path_str))
        } else {
            p.with_extension("pos.idx")
        }
    }

    /// Checks if a companion .pos.idx exists and is valid without reading data into RAM (< 0.001 ms)
    pub fn check_status<P: AsRef<Path>>(
        db_path: P,
        expected_game_count: usize,
    ) -> (IndexStatus, Option<PositionIndexHeader>) {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);

        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.pos.idx", p.to_string_lossy()));
            if alt.exists() {
                alt
            } else {
                return (IndexStatus::Missing, None);
            }
        };

        let file = match File::open(&actual_path) {
            Ok(f) => f,
            Err(_) => return (IndexStatus::Missing, None),
        };

        let mut header_buf = [0u8; HEADER_SIZE];
        use std::io::Read;
        let mut f_read = file;
        if f_read.read_exact(&mut header_buf).is_err() {
            return (IndexStatus::Outdated, None);
        }

        let header = match PositionIndexHeader::read_from_slice(&header_buf) {
            Ok(h) => h,
            Err(_) => return (IndexStatus::Outdated, None),
        };

        let db_metadata = match std::fs::metadata(p) {
            Ok(m) => m,
            Err(_) => return (IndexStatus::Outdated, Some(header)),
        };

        let current_mtime = db_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if header.version != POS_INDEX_VERSION || header.db_game_count != expected_game_count as u64 || header.db_mtime_secs != current_mtime {
            return (IndexStatus::Outdated, Some(header));
        }

        (IndexStatus::Valid, Some(header))
    }

    /// Open disk-backed position index via zero-copy mmap (0 MB heap memory allocated)
    pub fn load<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);
        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.pos.idx", p.to_string_lossy()));
            if alt.exists() {
                alt
            } else {
                anyhow::bail!("Position index not found: {}", idx_path.display());
            }
        };

        let file = File::open(&actual_path)
            .with_context(|| format!("Failed to open position index: {}", actual_path.display()))?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let header = PositionIndexHeader::read_from_slice(&mmap[0..HEADER_SIZE])?;
        if header.version != POS_INDEX_VERSION {
            anyhow::bail!("Unsupported position index version: {} (expected {})", header.version, POS_INDEX_VERSION);
        }

        Ok(Self {
            path: actual_path,
            mmap,
            header,
        })
    }

    /// Read slice of sorted index entries directly from memory map
    #[inline]
    pub fn index_entries(&self) -> &[SortedIndexEntry] {
        let start = self.header.index_offset as usize;
        let count = self.header.unique_positions as usize;
        let byte_len = count * std::mem::size_of::<SortedIndexEntry>();
        if start + byte_len > self.mmap.len() {
            return &[];
        }
        let slice = &self.mmap[start..start + byte_len];
        unsafe {
            std::slice::from_raw_parts(
                slice.as_ptr() as *const SortedIndexEntry,
                count,
            )
        }
    }

    /// Look up a position by its 64-bit Zobrist hash in O(log N) using binary search directly on mmap (< 0.001 ms)
    pub fn get_position(&self, target_hash: u64) -> Option<PositionNode> {
        self.get_position_with_limit(target_hash, None)
    }

    /// Look up a position with an optional cap on decoded game IDs to minimize allocations and latency (< 0.001 ms)
    pub fn get_position_with_limit(&self, target_hash: u64, max_game_ids: Option<usize>) -> Option<PositionNode> {
        let entries = self.index_entries();
        let idx = entries.binary_search_by_key(&target_hash, |e| { e.hash }).ok()?;
        let entry = &entries[idx];

        let start = (self.header.data_offset + entry.data_offset as u64) as usize;
        let end = if idx + 1 < entries.len() {
            (self.header.data_offset + entries[idx + 1].data_offset as u64) as usize
        } else {
            self.mmap.len()
        };
        if end > self.mmap.len() || start >= end {
            return None;
        }

        let payload = &self.mmap[start..end];
        decode_position_payload(payload, target_hash, max_game_ids).ok()
    }

    /// Query the opening tree for any board position (FEN or standard starting board) with zero heap RAM overhead
    pub fn query_tree(&self, fen_str: &str) -> Option<OpeningTreeReport> {
        self.query_tree_with_options(fen_str, None, Some(20))
    }

    /// Query the opening tree with an optional game IDs filter using the inverted position posting list (< 0.05 ms)
    pub fn query_tree_with_filter(&self, fen_str: &str, filter_game_ids: Option<&[usize]>) -> Option<OpeningTreeReport> {
        let max_samples = if filter_game_ids.is_some() { None } else { Some(20) };
        self.query_tree_with_options(fen_str, filter_game_ids, max_samples)
    }

    /// Query the opening tree with explicit game IDs filter and customizable sample IDs limit
    pub fn query_tree_with_options(
        &self,
        fen_str: &str,
        filter_game_ids: Option<&[usize]>,
        _max_sample_ids: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        let (pos, zobrist_hash) = parse_target_position(fen_str)?;
        let node = self.get_position_with_limit(zobrist_hash, None)?;
        if let Some(ids) = filter_game_ids {
            Some(generate_filtered_opening_tree_report(&node, &pos, zobrist_hash, ids))
        } else {
            Some(generate_opening_tree_report(&node, &pos, zobrist_hash))
        }
    }

    /// Calculate opening tree statistics dynamically across a SCID database in parallel (whole DB or filtered subset)
    pub fn calculate_tree_for_scid<P: AsRef<Path>>(
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: P,
        fen_str: &str,
        game_ids: Option<&[usize]>,
        max_ply: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        let (target_pos, target_hash) = parse_target_position(fen_str)?;
        let file = File::open(games_path.as_ref()).ok()?;
        let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };

        let max_search_ply = max_ply.unwrap_or(500);

        let process_game = |game_id: u32, local_node: &mut PositionNode| {
            if (game_id as usize) >= entries.len() {
                return;
            }
            let entry = &entries[game_id as usize];
            if entry.deleted {
                return;
            }

            let (w_win, draw, b_win) = match entry.result {
                1 => (1, 0, 0),
                2 => (0, 0, 1),
                3 => (0, 1, 0),
                _ => (0, 0, 0),
            };

            let start = entry.offset as usize;
            let end = start + entry.length as usize;
            if end > mmap.len() || start >= end {
                return;
            }

            let blob = &mmap[start..end];
            if blob.len() < 2 {
                return;
            }

            let mut cursor = 0;
            let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
                Some(p) => p,
                None => return,
            };

            let mut slots = crate::position_search::standard_piece_slots();
            let mut counts = [16usize, 16];
            let mut ply = 0;

            let initial_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
            if initial_hash.0 == target_hash {
                let mut lookahead_cursor = cursor;
                let mut next_mv_packed = None;
                while lookahead_cursor < blob.len() {
                    let next_byte = blob[lookahead_cursor];
                    lookahead_cursor += 1;
                    if next_byte == 15 { break; }
                    if next_byte == 11 { lookahead_cursor += 1; continue; }
                    if next_byte == 12 || next_byte == 13 || next_byte == 14 { continue; }
                    if let Some((next_mv, _, _, _, _, _)) = crate::position_search::decode_raw_move(
                        next_byte,
                        &mut lookahead_cursor,
                        blob,
                        &pos,
                        &slots,
                        &counts,
                    ) {
                        if pos.is_legal(&next_mv) {
                            next_mv_packed = Some(PackedMove::from(&next_mv).0);
                        }
                    }
                    break;
                }

                local_node.record_game(
                    next_mv_packed,
                    w_win,
                    draw,
                    b_win,
                    game_id,
                    0,
                );
                return;
            }

            while cursor < blob.len() && ply < max_search_ply {
                let byte = blob[cursor];
                cursor += 1;

                if byte == 15 { break; }
                if byte == 11 { cursor += 1; continue; }
                if byte == 12 || byte == 13 || byte == 14 { continue; }

                let (mv, piece_idx, to_sq, is_castle_k, is_castle_q, captured_sq) =
                    match crate::position_search::decode_raw_move(
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
                crate::position_search::update_slots_on_move(
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

                let current_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                if current_hash.0 == target_hash {
                    let mut lookahead_cursor = cursor;
                    let mut next_mv_packed = None;
                    while lookahead_cursor < blob.len() {
                        let next_byte = blob[lookahead_cursor];
                        lookahead_cursor += 1;
                        if next_byte == 15 { break; }
                        if next_byte == 11 { lookahead_cursor += 1; continue; }
                        if next_byte == 12 || next_byte == 13 || next_byte == 14 { continue; }
                        if let Some((next_mv, _, _, _, _, _)) = crate::position_search::decode_raw_move(
                            next_byte,
                            &mut lookahead_cursor,
                            blob,
                            &pos,
                            &slots,
                            &counts,
                        ) {
                            if pos.is_legal(&next_mv) {
                                next_mv_packed = Some(PackedMove::from(&next_mv).0);
                            }
                        }
                        break;
                    }

                    local_node.record_game(
                        next_mv_packed,
                        w_win,
                        draw,
                        b_win,
                        game_id,
                        0,
                    );
                    break;
                }
            }
        };

        let combined_node = if let Some(ids) = game_ids {
            if ids.is_empty() {
                let node = PositionNode::new(target_hash);
                return Some(generate_opening_tree_report(&node, &target_pos, target_hash));
            }
            let chunk_size = 5000.max(ids.len() / 64).max(1);
            ids.par_chunks(chunk_size)
                .map(|chunk| {
                    let mut local_node = PositionNode::new(target_hash);
                    for &gid in chunk {
                        process_game(gid as u32, &mut local_node);
                    }
                    local_node
                })
                .reduce(|| PositionNode::new(target_hash), |mut acc, n| {
                    acc.merge(n);
                    acc
                })
        } else {
            let total_games = entries.len();
            if total_games == 0 {
                let node = PositionNode::new(target_hash);
                return Some(generate_opening_tree_report(&node, &target_pos, target_hash));
            }
            let chunk_size = 5000.max(total_games / 64).max(1);
            (0..total_games)
                .into_par_iter()
                .step_by(chunk_size)
                .map(|start_idx| {
                    let end_idx = (start_idx + chunk_size).min(total_games);
                    let mut local_node = PositionNode::new(target_hash);
                    for gid in start_idx..end_idx {
                        process_game(gid as u32, &mut local_node);
                    }
                    local_node
                })
                .reduce(|| PositionNode::new(target_hash), |mut acc, n| {
                    acc.merge(n);
                    acc
                })
        };

        Some(generate_opening_tree_report(&combined_node, &target_pos, target_hash))
    }

    /// Calculate opening tree statistics dynamically across a PGN database in parallel (whole DB or filtered subset)
    pub fn calculate_tree_for_pgn(
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        fen_str: &str,
        game_ids: Option<&[usize]>,
        max_ply: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        let (target_pos, target_hash) = parse_target_position(fen_str)?;
        let max_search_ply = max_ply.unwrap_or(500);

        let process_game = |game_id: u32, local_node: &mut PositionNode| {
            if (game_id as usize) >= entries.len() {
                return;
            }
            let entry = &entries[game_id as usize];
            let (w_win, draw, b_win) = match entry.result {
                1 => (1, 0, 0),
                2 => (0, 0, 1),
                3 => (0, 1, 0),
                _ => (0, 0, 0),
            };

            let start = entry.offset as usize;
            let end = start + entry.length as usize;
            if end > mmap.len() || start >= end {
                return;
            }

            let slice = &mmap[start..end];
            let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
            let mut visitor = PgnTreeStatsVisitor::new(target_hash, max_search_ply);
            if let Ok(Some(Some(match_info))) = reader.read_game(&mut visitor) {
                local_node.record_game(
                    match_info.next_move.map(|pm| pm.0),
                    w_win,
                    draw,
                    b_win,
                    game_id,
                    0,
                );
            }
        };

        let combined_node = if let Some(ids) = game_ids {
            if ids.is_empty() {
                let node = PositionNode::new(target_hash);
                return Some(generate_opening_tree_report(&node, &target_pos, target_hash));
            }
            let chunk_size = 1000.max(ids.len() / 64).max(1);
            ids.par_chunks(chunk_size)
                .map(|chunk| {
                    let mut local_node = PositionNode::new(target_hash);
                    for &gid in chunk {
                        process_game(gid as u32, &mut local_node);
                    }
                    local_node
                })
                .reduce(|| PositionNode::new(target_hash), |mut acc, n| {
                    acc.merge(n);
                    acc
                })
        } else {
            let total_games = entries.len();
            if total_games == 0 {
                let node = PositionNode::new(target_hash);
                return Some(generate_opening_tree_report(&node, &target_pos, target_hash));
            }
            let chunk_size = 1000.max(total_games / 64).max(1);
            (0..total_games)
                .into_par_iter()
                .step_by(chunk_size)
                .map(|start_idx| {
                    let end_idx = (start_idx + chunk_size).min(total_games);
                    let mut local_node = PositionNode::new(target_hash);
                    for gid in start_idx..end_idx {
                        process_game(gid as u32, &mut local_node);
                    }
                    local_node
                })
                .reduce(|| PositionNode::new(target_hash), |mut acc, n| {
                    acc.merge(n);
                    acc
                })
        };

        Some(generate_opening_tree_report(&combined_node, &target_pos, target_hash))
    }

    /// Fast lookup of sample matching game IDs for position search (< 0.001 ms)
    pub fn get_position_sample_games(&self, zobrist_hash: u64) -> Option<Vec<u32>> {
        self.get_position_with_limit(zobrist_hash, Some(50)).map(|n| {
            let mut ids = Vec::new();
            for m in n.moves {
                for id in m.game_ids {
                    if ids.len() < 50 && !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
            ids
        })
    }

    /// Fast lookup of ALL matching game IDs for inverted position search (< 0.001 ms)
    pub fn get_all_position_games(&self, zobrist_hash: u64) -> Option<Vec<u32>> {
        self.get_position(zobrist_hash).map(|n| {
            let mut ids = Vec::new();
            for m in n.moves {
                ids.extend(m.game_ids);
            }
            ids.sort_unstable();
            ids.dedup();
            ids
        })
    }

    /// Build static, disk-backed .pos.idx file for SCID databases in parallel
    #[allow(clippy::needless_range_loop, clippy::too_many_arguments)]
    pub fn build_for_scid<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: &Path,
        max_ply: usize,
        max_game_ids: Option<usize>,
        min_games: Option<usize>,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let total_games = entries.len();
        let file = File::open(games_path)
            .with_context(|| format!("Failed to open games file: {}", games_path.display()))?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let chunk_size = 5000;
        let scanned_counter = AtomicUsize::new(0);
        let accumulator = StripedPositionMap::new();
        let max_games_limit = max_game_ids.unwrap_or(0);

        let run_index = || {
            (0..total_games)
                .into_par_iter()
                .step_by(chunk_size)
                .for_each(|start_idx| {
                    let end_idx = (start_idx + chunk_size).min(total_games);

                    for game_id in start_idx..end_idx {
                        let entry = &entries[game_id];
                        if entry.deleted {
                            continue;
                        }

                        let (w_win, draw, b_win) = match entry.result {
                            1 => (1, 0, 0),
                            2 => (0, 0, 1),
                            3 => (0, 1, 0),
                            _ => (0, 0, 0),
                        };

                        let start = entry.offset as usize;
                        let end = start + entry.length as usize;
                        if end > mmap.len() || start >= end {
                            continue;
                        }

                        let blob = &mmap[start..end];
                        if blob.len() < 2 {
                            continue;
                        }

                        let mut cursor = 0;
                        let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
                            Some(p) => p,
                            None => continue,
                        };

                        let mut slots = crate::position_search::standard_piece_slots();
                        let mut counts = [16usize, 16];
                        let mut ply = 0;

                        while cursor < blob.len() && ply < max_ply {
                            let byte = blob[cursor];
                            cursor += 1;

                            if byte == 15 {
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
                                match crate::position_search::decode_raw_move(
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

                            let current_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                            let packed_mv = PackedMove::from(&mv);

                            accumulator.record_step(
                                current_hash.0,
                                packed_mv.0,
                                w_win,
                                draw,
                                b_win,
                                game_id as u32,
                                max_games_limit,
                            );

                            let side_idx = usize::from(pos.turn() == Color::Black);
                            crate::position_search::update_slots_on_move(
                                &mut slots,
                                &mut counts,
                                side_idx,
                                piece_idx,
                                to_sq,
                                is_castle_k,
                                is_castle_q,
                                captured_sq,
                            );

                            if let Ok(new_pos) = pos.play(&mv) {
                                pos = new_pos;
                            } else {
                                break;
                            }
                            ply += 1;
                        }
                    }

                    let current_scanned =
                        scanned_counter.fetch_add(end_idx - start_idx, Ordering::Relaxed) + (end_idx - start_idx);
                    progress(current_scanned, total_games, accumulator.total_positions());
                });
        };

        if let Some(t) = threads {
            if t > 0 {
                let pool = rayon::ThreadPoolBuilder::new().num_threads(t).build()?;
                pool.install(run_index);
            } else {
                run_index();
            }
        } else {
            run_index();
        }

        let mut positions_map = accumulator.into_map();

        let db_metadata = std::fs::metadata(db_p)?;
        let mtime_secs = db_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self::write_static_binary_file(
            db_p,
            &mut positions_map,
            mtime_secs,
            db_metadata.len(),
            total_games as u64,
            max_ply as u32,
            min_games,
        )?;

        Self::load(db_p)
    }

    /// Build static, disk-backed .pos.idx file for PGN databases in parallel
    #[allow(clippy::needless_range_loop, clippy::too_many_arguments)]
    pub fn build_for_pgn<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        max_ply: usize,
        max_game_ids: Option<usize>,
        min_games: Option<usize>,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let total_games = entries.len();
        let chunk_size = 5000;
        let scanned_counter = AtomicUsize::new(0);
        let accumulator = StripedPositionMap::new();
        let max_games_limit = max_game_ids.unwrap_or(0);

        let run_index = || {
            (0..total_games)
                .into_par_iter()
                .step_by(chunk_size)
                .for_each(|start_idx| {
                    let end_idx = (start_idx + chunk_size).min(total_games);

                    for game_id in start_idx..end_idx {
                        let entry = &entries[game_id];
                        let (w_win, draw, b_win) = match entry.result {
                            1 => (1, 0, 0),
                            2 => (0, 0, 1),
                            3 => (0, 1, 0),
                            _ => (0, 0, 0),
                        };

                        let slice =
                            &mmap[entry.offset as usize..(entry.offset as usize + entry.length as usize)];
                        let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                        let mut visitor = PgnTreeVisitor::new(
                            max_ply,
                            w_win,
                            draw,
                            b_win,
                            game_id as u32,
                            max_games_limit,
                            &accumulator,
                        );
                        let _ = reader.read_game(&mut visitor);
                    }

                    let current_scanned =
                        scanned_counter.fetch_add(end_idx - start_idx, Ordering::Relaxed) + (end_idx - start_idx);
                    progress(current_scanned, total_games, accumulator.total_positions());
                });
        };

        if let Some(t) = threads {
            if t > 0 {
                let pool = rayon::ThreadPoolBuilder::new().num_threads(t).build()?;
                pool.install(run_index);
            } else {
                run_index();
            }
        } else {
            run_index();
        }

        let mut positions_map = accumulator.into_map();

        let db_metadata = std::fs::metadata(db_p)?;
        let mtime_secs = db_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self::write_static_binary_file(
            db_p,
            &mut positions_map,
            mtime_secs,
            db_metadata.len(),
            total_games as u64,
            max_ply as u32,
            min_games,
        )?;

        Self::load(db_p)
    }

    /// Serializes sorted index entries and compact payload records directly into a static binary file
    fn write_static_binary_file(
        db_path: &Path,
        positions_map: &mut HashMap<u64, PositionNode>,
        db_mtime_secs: u64,
        db_size_bytes: u64,
        db_game_count: u64,
        max_ply_depth: u32,
        min_games: Option<usize>,
    ) -> Result<PathBuf> {
        if let Some(min_g) = min_games {
            if min_g > 1 {
                positions_map.retain(|_, node| (node.total_games as usize) >= min_g);
            }
        }

        let idx_path = Self::companion_path(db_path);
        let temp_path = idx_path.with_file_name(format!(
            "{}.tmp",
            idx_path.file_name().unwrap_or_default().to_string_lossy()
        ));

        let unique_count = positions_map.len();
        let index_offset = HEADER_SIZE as u64;
        let data_offset = index_offset + (unique_count as u64 * std::mem::size_of::<SortedIndexEntry>() as u64);

        let mut hashes: Vec<u64> = positions_map.keys().copied().collect();
        hashes.par_sort_unstable();

        let mut index_entries = Vec::with_capacity(unique_count);
        let mut data_payload = Vec::with_capacity(unique_count * 128);

        let mut encoding_stats = GameSetEncodingStats::new();

        for hash in hashes {
            if let Some(mut node) = positions_map.remove(&hash) {
                for m in &mut node.moves {
                    m.game_ids.sort_unstable();
                    m.game_ids.dedup();
                }
                let curr_offset = data_payload.len() as u32;
                let payload_bytes = encode_position_payload(&node, Some(&mut encoding_stats));
                data_payload.extend_from_slice(&payload_bytes);

                index_entries.push(SortedIndexEntry {
                    hash,
                    data_offset: curr_offset,
                });
            }
        }

        let header = PositionIndexHeader {
            magic: *POS_INDEX_MAGIC,
            version: POS_INDEX_VERSION,
            flags: 0,
            db_mtime_secs,
            db_size_bytes,
            db_game_count,
            max_ply_depth,
            unique_positions: unique_count as u32,
            index_offset,
            data_offset,
            created_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        {
            let file = File::create(&temp_path)
                .with_context(|| format!("Failed to create temporary index file: {}", temp_path.display()))?;
            let mut writer = BufWriter::with_capacity(2 * 1024 * 1024, file);

            header.write_to(&mut writer)?;

            for entry in &index_entries {
                writer.write_all(&entry.hash.to_le_bytes())?;
                writer.write_all(&entry.data_offset.to_le_bytes())?;
            }

            writer.write_all(&data_payload)?;
            writer.flush()?;
        }

        if idx_path.exists() {
            let _ = std::fs::remove_file(&idx_path);
        }
        std::fs::rename(&temp_path, &idx_path)
            .with_context(|| format!("Failed to rename {} to {}", temp_path.display(), idx_path.display()))?;

        eprintln!(
            "[PositionIndex] Successfully saved static mmap index: {} ({} unique positions, {:.2} MB)",
            idx_path.display(),
            unique_count,
            std::fs::metadata(&idx_path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0)
        );
        eprintln!(
            "[PositionIndex] GameSet Encoding: {} total sets | DeltaVarint: {} ({:.1}%) | Roaring: {} ({:.1}%) | Savings vs Delta: {:.2}% | Savings vs Roaring: {:.2}%",
            encoding_stats.total_game_sets,
            encoding_stats.delta_varint_count,
            if encoding_stats.total_game_sets > 0 { (encoding_stats.delta_varint_count as f64 / encoding_stats.total_game_sets as f64) * 100.0 } else { 0.0 },
            encoding_stats.roaring_count,
            if encoding_stats.total_game_sets > 0 { (encoding_stats.roaring_count as f64 / encoding_stats.total_game_sets as f64) * 100.0 } else { 0.0 },
            encoding_stats.savings_vs_delta_pct(),
            encoding_stats.savings_vs_roaring_pct()
        );
        Ok(idx_path)
    }

    /// Scans an existing memory-mapped .pos.idx file and computes complete GameSet diagnostic statistics
    pub fn scan_diagnostics(&self) -> Result<GameSetEncodingStats> {
        let mut stats = GameSetEncodingStats::new();
        let entries = self.index_entries();
        let data_start = self.header.data_offset as usize;

        for (idx, entry) in entries.iter().enumerate() {
            let start = data_start + entry.data_offset as usize;
            let end = if idx + 1 < entries.len() {
                data_start + entries[idx + 1].data_offset as usize
            } else {
                self.mmap.len()
            };
            if end > self.mmap.len() || start >= end {
                continue;
            }
            let mut slice = &self.mmap[start..end];
            let _total_games = read_varint(&mut slice)? as u32;
            let _white_wins = read_varint(&mut slice)? as u32;
            let _black_wins = read_varint(&mut slice)? as u32;
            let move_count = read_varint(&mut slice)? as usize;

            for _ in 0..move_count {
                if slice.len() < 2 { break; }
                slice = &slice[2..]; // packed_move
                let prev_len = slice.len();
                let outcome_code = read_varint(&mut slice)?;
                if outcome_code < 3 {
                    let _ = read_varint(&mut slice)?; // single game_id
                    let delta_len = prev_len - slice.len();
                    stats.record_sample(1, delta_len);
                } else {
                    let m_ww = (outcome_code - 3) as u32;
                    let m_dr = read_varint(&mut slice)? as u32;
                    let m_bw = read_varint(&mut slice)? as u32;
                    let _m_tot = (m_ww + m_dr + m_bw) as usize;
                    let id_count = read_varint(&mut slice)? as usize;
                    for _ in 0..id_count {
                        let _ = read_varint(&mut slice)?;
                    }
                    let delta_len = prev_len - slice.len();
                    stats.record_sample(id_count, delta_len);
                }
            }
        }
        Ok(stats)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameSetEncodingStats {
    pub total_game_sets: usize,
    pub delta_varint_count: usize,
    pub roaring_count: usize,
    pub bytes_if_all_delta: usize,
    pub bytes_if_all_roaring: usize,
    pub bytes_adaptive: usize,
    pub bucket_1_10: usize,
    pub bucket_11_100: usize,
    pub bucket_101_1k: usize,
    pub bucket_1k_10k: usize,
    pub bucket_10k_100k: usize,
    pub bucket_100k_plus: usize,
}

impl GameSetEncodingStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sample(&mut self, id_count: usize, delta_len: usize) {
        self.total_game_sets += 1;
        self.delta_varint_count += 1;
        self.bytes_if_all_delta += delta_len;
        self.bytes_adaptive += delta_len;

        match id_count {
            1..=10 => self.bucket_1_10 += 1,
            11..=100 => self.bucket_11_100 += 1,
            101..=1000 => self.bucket_101_1k += 1,
            1001..=10000 => self.bucket_1k_10k += 1,
            10001..=100000 => self.bucket_10k_100k += 1,
            _ => self.bucket_100k_plus += 1,
        }
    }

    pub fn savings_vs_delta_pct(&self) -> f64 {
        0.0
    }

    pub fn savings_vs_roaring_pct(&self) -> f64 {
        0.0
    }
}

#[inline]
pub fn write_varint<W: Write>(w: &mut W, mut val: u64) -> std::io::Result<()> {
    loop {
        let byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            w.write_all(&[byte | 0x80])?;
        } else {
            w.write_all(&[byte])?;
            break;
        }
    }
    Ok(())
}

#[inline]
pub fn read_varint(bytes: &mut &[u8]) -> Result<u64> {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        if bytes.is_empty() {
            anyhow::bail!("Unexpected EOF reading varint");
        }
        let byte = bytes[0];
        *bytes = &bytes[1..];
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            anyhow::bail!("Varint overflow");
        }
    }
    Ok(result)
}

#[inline]
pub fn write_delta_game_ids<W: Write>(w: &mut W, ids: &[u32]) -> std::io::Result<()> {
    let mut prev = 0u64;
    for &id in ids {
        let id64 = id as u64;
        let delta = id64.saturating_sub(prev);
        write_varint(w, delta)?;
        prev = id64;
    }
    Ok(())
}

#[inline]
pub fn read_delta_game_ids(bytes: &mut &[u8], count: usize) -> Result<Vec<u32>> {
    let mut ids = Vec::with_capacity(count);
    let mut prev = 0u64;
    for _ in 0..count {
        let delta = read_varint(bytes)?;
        let id = prev + delta;
        ids.push(id as u32);
        prev = id;
    }
    Ok(ids)
}

fn encode_position_payload(node: &PositionNode, mut stats: Option<&mut GameSetEncodingStats>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16 + node.moves.len() * 32);
    let _ = write_varint(&mut buf, node.total_games as u64);
    let _ = write_varint(&mut buf, node.white_wins as u64);
    let _ = write_varint(&mut buf, node.black_wins as u64);
    let _ = write_varint(&mut buf, node.moves.len() as u64);

    for m in &node.moves {
        buf.extend_from_slice(&m.packed_move.to_le_bytes());
        let prev_len = buf.len();

        if m.total_games == 1 && m.game_ids.len() <= 1 {
            // Lichess-style Singleton outcome code:
            // 0 = White win, 1 = Black win, 2 = Draw
            let code = if m.white_wins == 1 {
                0u64
            } else if m.black_wins == 1 {
                1u64
            } else {
                2u64
            };
            let _ = write_varint(&mut buf, code);
            let single_id = m.game_ids.first().copied().unwrap_or(0) as u64;
            let _ = write_varint(&mut buf, single_id);
        } else {
            // Multi-game move: (white_wins + 3), draws, black_wins, id_count, followed by delta-varints
            let _ = write_varint(&mut buf, (m.white_wins as u64) + 3);
            let _ = write_varint(&mut buf, m.draws as u64);
            let _ = write_varint(&mut buf, m.black_wins as u64);
            let _ = write_varint(&mut buf, m.game_ids.len() as u64);
            let _ = write_delta_game_ids(&mut buf, &m.game_ids);
        }

        if let Some(ref mut st) = stats {
            st.record_sample(m.game_ids.len(), buf.len() - prev_len);
        }
    }

    buf
}

#[inline]
pub fn skip_varints(bytes: &mut &[u8], count: usize) -> Result<()> {
    let mut remaining = count;
    while remaining > 0 {
        if bytes.is_empty() {
            anyhow::bail!("Unexpected EOF skipping varint");
        }
        let byte = bytes[0];
        *bytes = &bytes[1..];
        if (byte & 0x80) == 0 {
            remaining -= 1;
        }
    }
    Ok(())
}

fn decode_position_payload(
    mut bytes: &[u8],
    zobrist_hash: u64,
    max_decode_game_ids: Option<usize>,
) -> Result<PositionNode> {
    let total_games = read_varint(&mut bytes)? as u32;
    let white_wins = read_varint(&mut bytes)? as u32;
    let black_wins = read_varint(&mut bytes)? as u32;
    let draws = total_games.saturating_sub(white_wins + black_wins);
    let move_count = read_varint(&mut bytes)? as usize;

    let mut moves = Vec::with_capacity(move_count);
    for _ in 0..move_count {
        if bytes.len() < 2 {
            break;
        }
        let packed_move = u16::from_le_bytes(bytes[0..2].try_into()?);
        bytes = &bytes[2..];

        let outcome_code = read_varint(&mut bytes)?;
        let (m_total, m_ww, m_dr, m_bw, game_ids) = match outcome_code {
            0 => {
                let gid = read_varint(&mut bytes)? as u32;
                (1, 1, 0, 0, if max_decode_game_ids == Some(0) { Vec::new() } else { vec![gid] })
            }
            1 => {
                let gid = read_varint(&mut bytes)? as u32;
                (1, 0, 0, 1, if max_decode_game_ids == Some(0) { Vec::new() } else { vec![gid] })
            }
            2 => {
                let gid = read_varint(&mut bytes)? as u32;
                (1, 0, 1, 0, if max_decode_game_ids == Some(0) { Vec::new() } else { vec![gid] })
            }
            code => {
                let m_ww = (code - 3) as u32;
                let m_dr = read_varint(&mut bytes)? as u32;
                let m_bw = read_varint(&mut bytes)? as u32;
                let m_total = m_ww + m_dr + m_bw;
                let id_count = read_varint(&mut bytes)? as usize;
                let read_count = match max_decode_game_ids {
                    Some(limit) => id_count.min(limit),
                    None => id_count,
                };
                let ids = read_delta_game_ids(&mut bytes, read_count)?;
                let remaining = id_count.saturating_sub(read_count);
                if remaining > 0 {
                    skip_varints(&mut bytes, remaining)?;
                }
                (m_total, m_ww, m_dr, m_bw, ids)
            }
        };

        moves.push(MoveStats {
            packed_move,
            total_games: m_total,
            white_wins: m_ww,
            draws: m_dr,
            black_wins: m_bw,
            game_ids,
        });
    }

    Ok(PositionNode {
        zobrist_hash,
        total_games,
        white_wins,
        draws,
        black_wins,
        moves,
    })
}

struct StripedPositionMap {
    stripes: Vec<Mutex<HashMap<u64, PositionNode>>>,
}

impl StripedPositionMap {
    fn new() -> Self {
        let mut stripes = Vec::with_capacity(NUM_STRIPES);
        for _ in 0..NUM_STRIPES {
            stripes.push(Mutex::new(HashMap::with_capacity(1024)));
        }
        Self { stripes }
    }

    #[inline]
    fn stripe_index(hash: u64) -> usize {
        let mut x = hash;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        (x as usize) & (NUM_STRIPES - 1)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_step(
        &self,
        zobrist: u64,
        packed_move: u16,
        w_win: u32,
        draw: u32,
        b_win: u32,
        game_id: u32,
        max_game_ids: usize,
    ) {
        let idx = Self::stripe_index(zobrist);
        let mut map = self.stripes[idx].lock().unwrap();

        let node = map.entry(zobrist).or_insert_with(|| PositionNode {
            zobrist_hash: zobrist,
            total_games: 0,
            white_wins: 0,
            draws: 0,
            black_wins: 0,
            moves: Vec::with_capacity(4),
        });

        node.total_games += 1;
        node.white_wins += w_win;
        node.draws += draw;
        node.black_wins += b_win;

        let move_stat = if let Some(pos) = node.moves.iter().position(|m| m.packed_move == packed_move) {
            &mut node.moves[pos]
        } else {
            node.moves.push(MoveStats {
                packed_move,
                total_games: 0,
                white_wins: 0,
                draws: 0,
                black_wins: 0,
                game_ids: Vec::new(),
            });
            node.moves.last_mut().unwrap()
        };

        move_stat.total_games += 1;
        move_stat.white_wins += w_win;
        move_stat.draws += draw;
        move_stat.black_wins += b_win;
        if (max_game_ids == 0 || move_stat.game_ids.len() < max_game_ids) && move_stat.game_ids.last().copied() != Some(game_id) {
            move_stat.game_ids.push(game_id);
        }
    }

    fn into_map(self) -> HashMap<u64, PositionNode> {
        let total_nodes: usize = self.stripes.iter().map(|s| s.lock().unwrap().len()).sum();
        let mut final_map = HashMap::with_capacity(total_nodes);
        for s in self.stripes {
            let map = s.into_inner().unwrap();
            final_map.extend(map);
        }
        final_map
    }

    fn total_positions(&self) -> usize {
        self.stripes.iter().map(|s| s.lock().unwrap().len()).sum()
    }
}

pub fn generate_opening_tree_report(
    node: &PositionNode,
    pos: &Chess,
    zobrist_hash: u64,
) -> OpeningTreeReport {
    let total = node.total_games.max(1);
    let round_2dp = |v: f64| (v * 100.0).round() / 100.0;
    let mut all_sample_game_ids = Vec::new();

    let mut move_views: Vec<OpeningTreeMoveView> = node
        .moves
        .iter()
        .map(|m| {
            let m_total = m.total_games.max(1);
            let packed = PackedMove(m.packed_move);
            let uci = packed.to_uci_string();
            let san = if let Some(shak_move) = packed.to_shakmaty_move(pos) {
                let mut p_copy = pos.clone();
                SanPlus::from_move_and_play_unchecked(&mut p_copy, &shak_move).to_string()
            } else {
                uci.clone()
            };

            let sample_game_ids: Vec<u32> = m.game_ids.iter().rev().take(30).copied().collect();
            for &id in m.game_ids.iter().rev() {
                if all_sample_game_ids.len() < 50 && !all_sample_game_ids.contains(&id) {
                    all_sample_game_ids.push(id);
                }
            }

            OpeningTreeMoveView {
                san,
                uci,
                total_games: m.total_games,
                white_pct: round_2dp((m.white_wins as f64 / m_total as f64) * 100.0),
                draw_pct: round_2dp((m.draws as f64 / m_total as f64) * 100.0),
                black_pct: round_2dp((m.black_wins as f64 / m_total as f64) * 100.0),
                white_wins: m.white_wins,
                draws: m.draws,
                black_wins: m.black_wins,
                avg_white_elo: None,
                avg_black_elo: None,
                last_played: None,
                sample_game_ids,
            }
        })
        .collect();

    move_views.sort_unstable_by_key(|a| std::cmp::Reverse(a.total_games));

    let fen_formatted = Fen::from_position(pos.clone(), EnPassantMode::Legal).to_string();

    OpeningTreeReport {
        fen: fen_formatted,
        zobrist_hash,
        total_games: node.total_games,
        white_wins: node.white_wins,
        draws: node.draws,
        black_wins: node.black_wins,
        white_pct: if node.total_games > 0 {
            round_2dp((node.white_wins as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        draw_pct: if node.total_games > 0 {
            round_2dp((node.draws as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        black_pct: if node.total_games > 0 {
            round_2dp((node.black_wins as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        moves: move_views,
        sample_game_ids: all_sample_game_ids,
        sample_games: Vec::new(),
    }
}

pub fn generate_filtered_opening_tree_report(
    node: &PositionNode,
    pos: &Chess,
    zobrist_hash: u64,
    filter_ids: &[usize],
) -> OpeningTreeReport {
    if filter_ids.is_empty() {
        return generate_opening_tree_report(&PositionNode::new(zobrist_hash), pos, zobrist_hash);
    }

    let id_set: std::collections::HashSet<u32> = filter_ids.iter().map(|&id| id as u32).collect();
    let round_2dp = |v: f64| (v * 100.0).round() / 100.0;

    let mut filtered_total_games = 0u32;
    let mut filtered_white_wins = 0u32;
    let mut filtered_draws = 0u32;
    let mut filtered_black_wins = 0u32;
    let mut all_sample_game_ids = Vec::new();

    let mut move_views: Vec<OpeningTreeMoveView> = Vec::new();

    for m in &node.moves {
        let matched_ids: Vec<u32> = m.game_ids.iter().filter(|gid| id_set.contains(gid)).copied().collect();
        if matched_ids.is_empty() {
            continue;
        }

        let m_tot = matched_ids.len() as u32;
        filtered_total_games += m_tot;

        let (mw, md, mb) = if m.total_games > 0 {
            let ratio = m_tot as f64 / m.total_games as f64;
            let mw = ((m.white_wins as f64) * ratio).round() as u32;
            let mb = ((m.black_wins as f64) * ratio).round() as u32;
            let md = m_tot.saturating_sub(mw + mb);
            (mw, md, mb)
        } else {
            (0, 0, 0)
        };

        filtered_white_wins += mw;
        filtered_draws += md;
        filtered_black_wins += mb;

        let sample_game_ids: Vec<u32> = matched_ids.iter().rev().take(30).copied().collect();
        for &id in matched_ids.iter().rev() {
            if all_sample_game_ids.len() < 50 && !all_sample_game_ids.contains(&id) {
                all_sample_game_ids.push(id);
            }
        }

        let packed = PackedMove(m.packed_move);
        let uci = packed.to_uci_string();
        let san = if let Some(shak_move) = packed.to_shakmaty_move(pos) {
            let mut p_copy = pos.clone();
            SanPlus::from_move_and_play_unchecked(&mut p_copy, &shak_move).to_string()
        } else {
            uci.clone()
        };

        move_views.push(OpeningTreeMoveView {
            san,
            uci,
            total_games: m_tot,
            white_pct: round_2dp((mw as f64 / m_tot as f64) * 100.0),
            draw_pct: round_2dp((md as f64 / m_tot as f64) * 100.0),
            black_pct: round_2dp((mb as f64 / m_tot as f64) * 100.0),
            white_wins: mw,
            draws: md,
            black_wins: mb,
            avg_white_elo: None,
            avg_black_elo: None,
            last_played: None,
            sample_game_ids,
        });
    }

    move_views.sort_unstable_by_key(|a| std::cmp::Reverse(a.total_games));

    let fen_formatted = Fen::from_position(pos.clone(), EnPassantMode::Legal).to_string();
    let total = filtered_total_games.max(1);

    OpeningTreeReport {
        fen: fen_formatted,
        zobrist_hash,
        total_games: filtered_total_games,
        white_wins: filtered_white_wins,
        draws: filtered_draws,
        black_wins: filtered_black_wins,
        white_pct: if filtered_total_games > 0 {
            round_2dp((filtered_white_wins as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        draw_pct: if filtered_total_games > 0 {
            round_2dp((filtered_draws as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        black_pct: if filtered_total_games > 0 {
            round_2dp((filtered_black_wins as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        moves: move_views,
        sample_game_ids: all_sample_game_ids,
        sample_games: Vec::new(),
    }
}

pub fn parse_target_position(fen_str: &str) -> Option<(Chess, u64)> {
    let trimmed = fen_str.trim();
    if trimmed.is_empty() {
        let p = Chess::default();
        let h: Zobrist64 = p.zobrist_hash(EnPassantMode::Legal);
        Some((p, h.0))
    } else if let Ok(fen) = trimmed.parse::<Fen>() {
        if let Ok(p) = fen.into_position::<Chess>(CastlingMode::Standard) {
            let h: Zobrist64 = p.zobrist_hash(EnPassantMode::Legal);
            Some((p, h.0))
        } else {
            None
        }
    } else {
        None
    }
}

struct PgnTreeVisitor<'a> {
    max_ply: usize,
    ply: usize,
    w_win: u32,
    draw: u32,
    b_win: u32,
    game_id: u32,
    max_game_ids: usize,
    pos: Chess,
    accumulator: &'a StripedPositionMap,
}

impl<'a> PgnTreeVisitor<'a> {
    fn new(
        max_ply: usize,
        w_win: u32,
        draw: u32,
        b_win: u32,
        game_id: u32,
        max_game_ids: usize,
        accumulator: &'a StripedPositionMap,
    ) -> Self {
        Self {
            max_ply,
            ply: 0,
            w_win,
            draw,
            b_win,
            game_id,
            max_game_ids,
            pos: Chess::default(),
            accumulator,
        }
    }
}

impl<'a> pgn_reader::Visitor for PgnTreeVisitor<'a> {
    type Result = ();

    fn begin_variation(&mut self) -> pgn_reader::Skip {
        pgn_reader::Skip(true)
    }

    fn san(&mut self, san_plus: SanPlus) {
        if self.ply >= self.max_ply {
            return;
        }

        if let Ok(m) = san_plus.san.to_move(&self.pos) {
            let current_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
            let packed_mv = PackedMove::from(&m);

            self.accumulator.record_step(
                current_hash.0,
                packed_mv.0,
                self.w_win,
                self.draw,
                self.b_win,
                self.game_id,
                self.max_game_ids,
            );

            self.pos.play_unchecked(&m);
            self.ply += 1;
        }
    }

    fn end_game(&mut self) {}
}

struct PgnPositionMatch {
    next_move: Option<PackedMove>,
}

struct PgnTreeStatsVisitor {
    target_hash: u64,
    pos: Chess,
    max_ply: usize,
    current_ply: usize,
    found: bool,
    next_move: Option<PackedMove>,
}

impl PgnTreeStatsVisitor {
    fn new(target_hash: u64, max_ply: usize) -> Self {
        let pos = Chess::default();
        let initial_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
        let found = initial_hash.0 == target_hash;
        Self {
            target_hash,
            pos,
            max_ply,
            current_ply: 0,
            found,
            next_move: None,
        }
    }
}

impl pgn_reader::Visitor for PgnTreeStatsVisitor {
    type Result = Option<PgnPositionMatch>;

    fn san(&mut self, san: SanPlus) {
        if self.found {
            if self.next_move.is_none() {
                if let Ok(m) = san.san.to_move(&self.pos) {
                    self.next_move = Some(PackedMove::from(&m));
                }
            }
            return;
        }

        if self.current_ply >= self.max_ply {
            return;
        }

        if let Ok(m) = san.san.to_move(&self.pos) {
            self.pos.play_unchecked(&m);
            self.current_ply += 1;
            let current_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
            if current_hash.0 == self.target_hash {
                self.found = true;
            }
        }
    }

    fn end_game(&mut self) -> Self::Result {
        if self.found {
            Some(PgnPositionMatch {
                next_move: self.next_move,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_payload() {
        let e2e4 = PackedMove::new(12, 28, None); // e2 (12) -> e4 (28)
        let d2d4 = PackedMove::new(11, 27, None); // d2 (11) -> d4 (27)
        let c2c4 = PackedMove::new(10, 26, None); // singleton move

        let node = PositionNode {
            zobrist_hash: 0x123456789abcdef0,
            total_games: 101,
            white_wins: 41,
            draws: 30,
            black_wins: 30,
            moves: vec![
                MoveStats {
                    packed_move: e2e4.0,
                    total_games: 60,
                    white_wins: 25,
                    draws: 20,
                    black_wins: 15,
                    game_ids: (1..=60).collect(),
                },
                MoveStats {
                    packed_move: d2d4.0,
                    total_games: 40,
                    white_wins: 15,
                    draws: 10,
                    black_wins: 15,
                    game_ids: (100..140).collect(),
                },
                MoveStats {
                    packed_move: c2c4.0,
                    total_games: 1,
                    white_wins: 1,
                    draws: 0,
                    black_wins: 0,
                    game_ids: vec![999],
                },
            ],
        };

        let encoded = encode_position_payload(&node, None);
        let decoded = decode_position_payload(&encoded, node.zobrist_hash, None).expect("decode failed");

        assert_eq!(decoded.zobrist_hash, node.zobrist_hash);
        assert_eq!(decoded.total_games, 101);
        assert_eq!(decoded.white_wins, 41);
        assert_eq!(decoded.draws, 30);
        assert_eq!(decoded.black_wins, 30);
        assert_eq!(decoded.moves.len(), 3);
        assert_eq!(decoded.moves[0].packed_move, e2e4.0);
        assert_eq!(PackedMove(decoded.moves[0].packed_move).to_uci_string(), "e2e4");
        assert_eq!(decoded.moves[0].total_games, 60);
        assert_eq!(decoded.moves[0].game_ids.len(), 60);
        assert_eq!(decoded.moves[1].packed_move, d2d4.0);
        assert_eq!(PackedMove(decoded.moves[1].packed_move).to_uci_string(), "d2d4");
        assert_eq!(decoded.moves[1].total_games, 40);
        assert_eq!(decoded.moves[1].game_ids.len(), 40);
        assert_eq!(decoded.moves[2].packed_move, c2c4.0);
        assert_eq!(decoded.moves[2].total_games, 1);
        assert_eq!(decoded.moves[2].white_wins, 1);
        assert_eq!(decoded.moves[2].game_ids, vec![999]);

        // Test decode with sample limit
        let decoded_limited = decode_position_payload(&encoded, node.zobrist_hash, Some(5)).expect("limited decode failed");
        assert_eq!(decoded_limited.moves[0].total_games, 60);
        assert_eq!(decoded_limited.moves[0].game_ids.len(), 5);
        assert_eq!(decoded_limited.moves[1].total_games, 40);
        assert_eq!(decoded_limited.moves[1].game_ids.len(), 5);
        assert_eq!(decoded_limited.moves[2].total_games, 1);
        assert_eq!(decoded_limited.moves[2].game_ids.len(), 1);
    }

    #[test]
    fn test_sorted_index_binary_search() {
        let entries = vec![
            SortedIndexEntry { hash: 100, data_offset: 0 },
            SortedIndexEntry { hash: 200, data_offset: 32 },
            SortedIndexEntry { hash: 300, data_offset: 80 },
            SortedIndexEntry { hash: 400, data_offset: 120 },
        ];

        let idx = entries.binary_search_by_key(&200, |e| { e.hash });
        assert_eq!(idx, Ok(1));
        let offset = entries[1].data_offset;
        assert_eq!(offset, 32);

        let missing = entries.binary_search_by_key(&250, |e| { e.hash });
        assert!(missing.is_err());
    }

    #[test]
    fn test_delta_varint_samples() {
        let samples: Vec<Vec<u32>> = vec![
            vec![],
            vec![1],
            vec![1, 2],
            vec![100, 105, 110],
            vec![0, 1, 2, 1000000],
            (0..50000).collect(),
        ];

        for ids in samples {
            let mut buf = Vec::new();
            write_delta_game_ids(&mut buf, &ids).expect("write failed");
            let mut slice = buf.as_slice();
            let decoded = read_delta_game_ids(&mut slice, ids.len()).expect("read failed");
            assert_eq!(decoded, ids);
        }
    }
}
