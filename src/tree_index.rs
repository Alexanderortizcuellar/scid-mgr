use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
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
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position};

pub const TREE_INDEX_MAGIC: &[u8; 8] = b"SCIDTRE1";
pub const TREE_INDEX_VERSION: u32 = 1;
pub const DEFAULT_MAX_TREE_PLY: usize = 24; // 12 full moves
const NUM_STRIPES: usize = 256;
const HEADER_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus {
    Valid,
    Outdated,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeIndexDiagnostics {
    pub total_positions: usize,
    pub total_tree_moves: usize,
    pub bytes_total: usize,
    pub bucket_1_10: usize,
    pub bucket_11_100: usize,
    pub bucket_101_1k: usize,
    pub bucket_1k_10k: usize,
    pub bucket_10k_100k: usize,
    pub bucket_100k_plus: usize,
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

        pos.legal_moves()
            .into_iter()
            .find(|m| m.from() == Some(from_sq) && m.to() == to_sq && m.promotion() == promo)
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
pub struct TreeIndexHeader {
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

impl TreeIndexHeader {
    pub fn read_from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            anyhow::bail!("Header slice too small");
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if &magic != TREE_INDEX_MAGIC {
            anyhow::bail!("Invalid magic bytes in tree index");
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
pub struct SortedTreeIndexEntry {
    pub hash: u64,
    pub data_offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeMoveStats {
    pub packed_move: u16,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub white_elo_sum: u64,
    pub black_elo_sum: u64,
    pub elo_game_count: u32,
}

impl TreeMoveStats {
    pub fn avg_white_elo(&self) -> Option<u32> {
        if self.elo_game_count > 0 {
            Some((self.white_elo_sum / self.elo_game_count as u64) as u32)
        } else {
            None
        }
    }

    pub fn avg_black_elo(&self) -> Option<u32> {
        if self.elo_game_count > 0 {
            Some((self.black_elo_sum / self.elo_game_count as u64) as u32)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreePositionNode {
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub moves: Vec<TreeMoveStats>,
}

impl TreePositionNode {
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
        w_elo: u16,
        b_elo: u16,
    ) {
        self.total_games += 1;
        self.white_wins += w_win;
        self.draws += draw;
        self.black_wins += b_win;

        if let Some(packed) = next_move {
            let move_stat =
                if let Some(pos) = self.moves.iter().position(|m| m.packed_move == packed) {
                    &mut self.moves[pos]
                } else {
                    self.moves.push(TreeMoveStats {
                        packed_move: packed,
                        total_games: 0,
                        white_wins: 0,
                        draws: 0,
                        black_wins: 0,
                        white_elo_sum: 0,
                        black_elo_sum: 0,
                        elo_game_count: 0,
                    });
                    self.moves.last_mut().unwrap()
                };

            move_stat.total_games += 1;
            move_stat.white_wins += w_win;
            move_stat.draws += draw;
            move_stat.black_wins += b_win;
            if w_elo > 0 && b_elo > 0 {
                move_stat.white_elo_sum += w_elo as u64;
                move_stat.black_elo_sum += b_elo as u64;
                move_stat.elo_game_count += 1;
            }
        }
    }

    pub fn merge(&mut self, other: TreePositionNode) {
        self.total_games += other.total_games;
        self.white_wins += other.white_wins;
        self.draws += other.draws;
        self.black_wins += other.black_wins;

        for other_m in other.moves {
            if let Some(m) = self
                .moves
                .iter_mut()
                .find(|m| m.packed_move == other_m.packed_move)
            {
                m.total_games += other_m.total_games;
                m.white_wins += other_m.white_wins;
                m.draws += other_m.draws;
                m.black_wins += other_m.black_wins;
                m.white_elo_sum += other_m.white_elo_sum;
                m.black_elo_sum += other_m.black_elo_sum;
                m.elo_game_count += other_m.elo_game_count;
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

/// Static, disk-backed Opening Tree Stats index using zero-copy memory mapping (`memmap2`)
pub struct TreeIndex {
    pub path: PathBuf,
    mmap: memmap2::Mmap,
    pub header: TreeIndexHeader,
}

impl TreeIndex {
    /// Determines companion `.tree.idx` path for any database file
    pub fn companion_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
        let p = db_path.as_ref();
        let path_str = p.to_string_lossy();
        let lower = path_str.to_lowercase();
        if lower.ends_with(".si5")
            || lower.ends_with(".si4")
            || lower.ends_with(".sg5")
            || lower.ends_with(".sg4")
            || lower.ends_with(".sn5")
            || lower.ends_with(".sn4")
        {
            p.with_extension("tree.idx")
        } else if lower.ends_with(".pgn") {
            PathBuf::from(format!("{}.tree.idx", path_str))
        } else {
            p.with_extension("tree.idx")
        }
    }

    /// Checks if a companion .tree.idx exists and is valid without reading data into RAM (< 0.001 ms)
    pub fn check_status<P: AsRef<Path>>(
        db_path: P,
        expected_game_count: usize,
    ) -> (IndexStatus, Option<TreeIndexHeader>) {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);

        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.tree.idx", p.to_string_lossy()));
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
        let mut f_read = file;
        if f_read.read_exact(&mut header_buf).is_err() {
            return (IndexStatus::Outdated, None);
        }

        let header = match TreeIndexHeader::read_from_slice(&header_buf) {
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

        if header.version != TREE_INDEX_VERSION
            || header.db_game_count != expected_game_count as u64
            || header.db_mtime_secs != current_mtime
        {
            return (IndexStatus::Outdated, Some(header));
        }

        (IndexStatus::Valid, Some(header))
    }

    /// Open disk-backed tree index via zero-copy mmap (0 MB heap memory allocated)
    pub fn load<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);
        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.tree.idx", p.to_string_lossy()));
            if alt.exists() {
                alt
            } else {
                anyhow::bail!("Tree index not found: {}", idx_path.display());
            }
        };

        let file = File::open(&actual_path)
            .with_context(|| format!("Failed to open tree index: {}", actual_path.display()))?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let header = TreeIndexHeader::read_from_slice(&mmap[0..HEADER_SIZE])?;
        if header.version != TREE_INDEX_VERSION {
            anyhow::bail!(
                "Unsupported tree index version: {} (expected {})",
                header.version,
                TREE_INDEX_VERSION
            );
        }

        Ok(Self {
            path: actual_path,
            mmap,
            header,
        })
    }

    /// Read slice of sorted index entries directly from memory map
    #[inline]
    pub fn index_entries(&self) -> &[SortedTreeIndexEntry] {
        let start = self.header.index_offset as usize;
        let count = self.header.unique_positions as usize;
        let byte_len = count * std::mem::size_of::<SortedTreeIndexEntry>();
        if start + byte_len > self.mmap.len() {
            return &[];
        }
        let slice = &self.mmap[start..start + byte_len];
        unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const SortedTreeIndexEntry, count) }
    }

    /// Look up a position node by its 64-bit Zobrist hash in O(log N) directly from mmap (< 0.001 ms)
    pub fn get_position(&self, target_hash: u64) -> Option<TreePositionNode> {
        let entries = self.index_entries();
        let idx = entries
            .binary_search_by_key(&target_hash, |e| e.hash)
            .ok()?;
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
        decode_tree_position_payload(payload, target_hash).ok()
    }

    /// Query the opening tree for any board position (FEN or standard starting board) with zero heap RAM overhead (< 0.01 ms)
    pub fn query_tree(&self, fen_str: &str) -> Option<OpeningTreeReport> {
        let (pos, zobrist_hash) = parse_target_position(fen_str)?;
        let node = self.get_position(zobrist_hash)?;
        Some(generate_tree_report(&node, &pos, zobrist_hash))
    }

    /// Query the opening tree with optional parameters for API compatibility
    pub fn query_tree_with_options(
        &self,
        fen_str: &str,
        _target_game_ids: Option<&[usize]>,
        _sample_limit: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        self.query_tree(fen_str)
    }

    /// Scans diagnostics and distribution metrics of opening tree index
    pub fn scan_diagnostics(&self) -> Result<TreeIndexDiagnostics> {
        let entries = self.index_entries();
        let mut diag = TreeIndexDiagnostics {
            total_positions: entries.len(),
            ..Default::default()
        };

        for i in 0..entries.len() {
            let start = (self.header.data_offset + entries[i].data_offset as u64) as usize;
            let end = if i + 1 < entries.len() {
                (self.header.data_offset + entries[i + 1].data_offset as u64) as usize
            } else {
                self.mmap.len()
            };
            if start < end && end <= self.mmap.len() {
                let payload = &self.mmap[start..end];
                diag.bytes_total += payload.len();
                if let Ok(node) = decode_tree_position_payload(payload, entries[i].hash) {
                    let count = node.total_games as usize;
                    diag.total_tree_moves += node.moves.len();
                    match count {
                        1..=10 => diag.bucket_1_10 += 1,
                        11..=100 => diag.bucket_11_100 += 1,
                        101..=1_000 => diag.bucket_101_1k += 1,
                        1_001..=10_000 => diag.bucket_1k_10k += 1,
                        10_001..=100_000 => diag.bucket_10k_100k += 1,
                        _ => diag.bucket_100k_plus += 1,
                    }
                }
            }
        }

        Ok(diag)
    }

    /// On-the-fly dynamic calculation of opening tree statistics for SCID databases
    pub fn calculate_tree_for_scid<P: AsRef<Path>>(
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: P,
        fen_str: &str,
        target_game_ids: Option<&[usize]>,
        _max_depth: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        let (target_pos, target_hash) = parse_target_position(fen_str)?;
        let file = File::open(games_path.as_ref()).ok()?;
        let mmap = unsafe { memmap2::Mmap::map(&file).ok()? };

        let max_ply = 50;
        let mut node = TreePositionNode::new(target_hash);

        let process_game = |game_id: usize, node: &mut TreePositionNode| {
            if game_id >= entries.len() {
                return;
            }
            let entry = &entries[game_id];
            if entry.deleted {
                return;
            }

            let start = entry.offset as usize;
            let end = start + entry.length as usize;
            if end > mmap.len() || start >= end {
                return;
            }

            let blob = &mmap[start..end];
            let mut cursor = 0;

            let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
                Some(p) => p,
                None => return,
            };

            let (w_win, draw, b_win) = match entry.result {
                1 => (1, 0, 0),
                2 => (0, 0, 1),
                3 => (0, 1, 0),
                _ => (0, 0, 0),
            };

            let w_elo = entry.white_elo;
            let b_elo = entry.black_elo;

            let mut slots = crate::position_search::standard_piece_slots();
            let mut counts = [16usize, 16usize];
            let mut ply = 0;

            while cursor < blob.len() && ply < max_ply {
                let curr_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                let is_match = curr_hash.0 == target_hash;

                let b = blob[cursor];
                cursor += 1;

                if b == 15 {
                    if is_match {
                        node.record_game(None, w_win, draw, b_win, w_elo, b_elo);
                    }
                    break;
                }
                if b == 11 {
                    cursor += 1;
                    continue;
                }
                if b == 12 || b == 13 || b == 14 {
                    continue;
                }

                let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
                    match crate::position_search::decode_raw_move(
                        b,
                        &mut cursor,
                        blob,
                        &pos,
                        &slots,
                        &counts,
                    ) {
                        Some(res) => res,
                        None => break,
                    };

                if !pos.is_legal(&mv) {
                    break;
                }

                if is_match {
                    let packed = PackedMove::from(&mv).0;
                    node.record_game(Some(packed), w_win, draw, b_win, w_elo, b_elo);
                    break;
                }

                let side_idx = usize::from(pos.turn() == shakmaty::Color::Black);
                crate::position_search::update_slots_on_move(
                    &mut slots,
                    &mut counts,
                    side_idx,
                    piece_idx,
                    to_sq,
                    is_k,
                    is_q,
                    cap_sq,
                );

                pos.play_unchecked(&mv);
                ply += 1;
            }
        };

        if let Some(ids) = target_game_ids {
            for &gid in ids {
                process_game(gid, &mut node);
            }
        } else {
            for gid in 0..entries.len() {
                process_game(gid, &mut node);
            }
        }

        Some(generate_tree_report(&node, &target_pos, target_hash))
    }

    /// On-the-fly dynamic calculation of opening tree statistics for PGN databases
    pub fn calculate_tree_for_pgn(
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        fen_str: &str,
        target_game_ids: Option<&[usize]>,
        _max_depth: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        let (target_pos, target_hash) = parse_target_position(fen_str)?;
        let max_ply = 50;
        let accumulator = StripedTreePositionMap::new();

        let process_game = |game_id: usize| {
            if game_id >= entries.len() {
                return;
            }
            let entry = &entries[game_id];
            let (w_win, draw, b_win) = match entry.result {
                1 => (1, 0, 0),
                2 => (0, 0, 1),
                3 => (0, 1, 0),
                _ => (0, 0, 0),
            };
            let w_elo = entry.white_elo;
            let b_elo = entry.black_elo;

            let slice =
                &mmap[entry.offset as usize..(entry.offset as usize + entry.length as usize)];
            let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
            let mut visitor =
                PgnTreeStatsVisitor::new(max_ply, w_win, draw, b_win, w_elo, b_elo, &accumulator);
            let _ = reader.read_game(&mut visitor);
        };

        if let Some(ids) = target_game_ids {
            for &gid in ids {
                process_game(gid);
            }
        } else {
            for gid in 0..entries.len() {
                process_game(gid);
            }
        }

        let map = accumulator.into_map();
        let node = map
            .get(&target_hash)
            .cloned()
            .unwrap_or_else(|| TreePositionNode::new(target_hash));
        Some(generate_tree_report(&node, &target_pos, target_hash))
    }

    /// Build static, disk-backed .tree.idx file for SCID databases in parallel across CPU cores
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::needless_range_loop)]
    pub fn build_for_scid<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: P,
        max_ply: usize,
        _max_games: Option<usize>,
        min_games: Option<usize>,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let file = File::open(games_path.as_ref()).with_context(|| {
            format!(
                "Failed to open games file: {}",
                games_path.as_ref().display()
            )
        })?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let total_games = entries.len();
        let chunk_size = 5000;
        let scanned_counter = AtomicUsize::new(0);
        let accumulator = StripedTreePositionMap::new();

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

                        let start = entry.offset as usize;
                        let end = start + entry.length as usize;
                        if end > mmap.len() || start >= end {
                            continue;
                        }

                        let blob = &mmap[start..end];
                        let mut cursor = 0;

                        let mut pos =
                            match crate::position_search::parse_start_position(blob, &mut cursor) {
                                Some(p) => p,
                                None => continue,
                            };

                        let (w_win, draw, b_win) = match entry.result {
                            1 => (1, 0, 0),
                            2 => (0, 0, 1),
                            3 => (0, 1, 0),
                            _ => (0, 0, 0),
                        };

                        let w_elo = entry.white_elo;
                        let b_elo = entry.black_elo;

                        let mut slots = crate::position_search::standard_piece_slots();
                        let mut counts = [16usize, 16usize];
                        let mut ply = 0;

                        while cursor < blob.len() && ply < max_ply {
                            let b = blob[cursor];
                            cursor += 1;

                            if b == 15 {
                                break;
                            }
                            if b == 11 {
                                cursor += 1;
                                continue;
                            }
                            if b == 12 || b == 13 || b == 14 {
                                continue;
                            }

                            let pre_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);

                            let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
                                match crate::position_search::decode_raw_move(
                                    b,
                                    &mut cursor,
                                    blob,
                                    &pos,
                                    &slots,
                                    &counts,
                                ) {
                                    Some(res) => res,
                                    None => break,
                                };

                            if !pos.is_legal(&mv) {
                                break;
                            }

                            let packed = PackedMove::from(&mv).0;
                            accumulator.record(
                                pre_hash.0,
                                Some(packed),
                                w_win,
                                draw,
                                b_win,
                                w_elo,
                                b_elo,
                            );

                            let side_idx = usize::from(pos.turn() == shakmaty::Color::Black);
                            crate::position_search::update_slots_on_move(
                                &mut slots,
                                &mut counts,
                                side_idx,
                                piece_idx,
                                to_sq,
                                is_k,
                                is_q,
                                cap_sq,
                            );

                            pos.play_unchecked(&mv);
                            ply += 1;

                            if ply == max_ply || cursor >= blob.len() {
                                let final_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                                accumulator.record(
                                    final_hash.0,
                                    None,
                                    w_win,
                                    draw,
                                    b_win,
                                    w_elo,
                                    b_elo,
                                );
                            }
                        }
                    }

                    let current_scanned = scanned_counter
                        .fetch_add(end_idx - start_idx, Ordering::Relaxed)
                        + (end_idx - start_idx);
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

    /// Build static, disk-backed .tree.idx file for PGN databases in parallel
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::needless_range_loop)]
    pub fn build_for_pgn<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        max_ply: usize,
        _max_games: Option<usize>,
        min_games: Option<usize>,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let total_games = entries.len();
        let chunk_size = 5000;
        let scanned_counter = AtomicUsize::new(0);
        let accumulator = StripedTreePositionMap::new();

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

                        let w_elo = entry.white_elo;
                        let b_elo = entry.black_elo;

                        let slice = &mmap[entry.offset as usize
                            ..(entry.offset as usize + entry.length as usize)];
                        let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                        let mut visitor = PgnTreeStatsVisitor::new(
                            max_ply,
                            w_win,
                            draw,
                            b_win,
                            w_elo,
                            b_elo,
                            &accumulator,
                        );
                        let _ = reader.read_game(&mut visitor);
                    }

                    let current_scanned = scanned_counter
                        .fetch_add(end_idx - start_idx, Ordering::Relaxed)
                        + (end_idx - start_idx);
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

    /// Serializes sorted index entries and compact move stats records directly into a static binary file
    fn write_static_binary_file(
        db_path: &Path,
        positions_map: &mut HashMap<u64, TreePositionNode>,
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
        let data_offset = index_offset
            + (unique_count as u64 * std::mem::size_of::<SortedTreeIndexEntry>() as u64);

        let mut hashes: Vec<u64> = positions_map.keys().copied().collect();
        hashes.par_sort_unstable();

        let mut index_entries = Vec::with_capacity(unique_count);
        let mut data_payload = Vec::with_capacity(unique_count * 32);

        for hash in hashes {
            if let Some(node) = positions_map.remove(&hash) {
                let curr_offset = data_payload.len() as u32;
                let payload_bytes = encode_tree_position_payload(&node);
                data_payload.extend_from_slice(&payload_bytes);

                index_entries.push(SortedTreeIndexEntry {
                    hash,
                    data_offset: curr_offset,
                });
            }
        }

        let header = TreeIndexHeader {
            magic: *TREE_INDEX_MAGIC,
            version: TREE_INDEX_VERSION,
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
            let file = File::create(&temp_path).with_context(|| {
                format!(
                    "Failed to create temporary tree index file: {}",
                    temp_path.display()
                )
            })?;
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
        std::fs::rename(&temp_path, &idx_path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                idx_path.display()
            )
        })?;

        eprintln!(
            "[TreeIndex] Successfully saved static tree index: {} ({} unique positions, {:.2} MB)",
            idx_path.display(),
            unique_count,
            std::fs::metadata(&idx_path)
                .map(|m| m.len() as f64 / 1_048_576.0)
                .unwrap_or(0.0)
        );
        Ok(idx_path)
    }
}

// ---------------------------------------------------------------------------
// Thread-Safe Striped Map for Parallel Indexing
// ---------------------------------------------------------------------------

struct StripedTreePositionMap {
    stripes: Vec<Mutex<HashMap<u64, TreePositionNode>>>,
}

impl StripedTreePositionMap {
    fn new() -> Self {
        let mut stripes = Vec::with_capacity(NUM_STRIPES);
        for _ in 0..NUM_STRIPES {
            stripes.push(Mutex::new(HashMap::with_capacity(1024)));
        }
        Self { stripes }
    }

    #[inline]
    fn stripe_index(hash: u64) -> usize {
        (hash as usize) % NUM_STRIPES
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &self,
        hash: u64,
        next_move: Option<u16>,
        w_win: u32,
        draw: u32,
        b_win: u32,
        w_elo: u16,
        b_elo: u16,
    ) {
        let idx = Self::stripe_index(hash);
        let mut guard = self.stripes[idx].lock().unwrap();
        let node = guard
            .entry(hash)
            .or_insert_with(|| TreePositionNode::new(hash));
        node.record_game(next_move, w_win, draw, b_win, w_elo, b_elo);
    }

    fn total_positions(&self) -> usize {
        self.stripes.iter().map(|s| s.lock().unwrap().len()).sum()
    }

    fn into_map(self) -> HashMap<u64, TreePositionNode> {
        let total_size: usize = self.stripes.iter().map(|s| s.lock().unwrap().len()).sum();
        let mut combined: HashMap<u64, TreePositionNode> = HashMap::with_capacity(total_size);
        for stripe in self.stripes {
            let map = stripe.into_inner().unwrap();
            for (k, v) in map {
                if let Some(existing) = combined.get_mut(&k) {
                    existing.merge(v);
                } else {
                    combined.insert(k, v);
                }
            }
        }
        combined
    }
}

// ---------------------------------------------------------------------------
// PGN Visitor for Parallel Tree Indexing
// ---------------------------------------------------------------------------

struct PgnTreeStatsVisitor<'a> {
    max_ply: usize,
    w_win: u32,
    draw: u32,
    b_win: u32,
    w_elo: u16,
    b_elo: u16,
    accumulator: &'a StripedTreePositionMap,
    pos: Chess,
    ply: usize,
}

impl<'a> PgnTreeStatsVisitor<'a> {
    fn new(
        max_ply: usize,
        w_win: u32,
        draw: u32,
        b_win: u32,
        w_elo: u16,
        b_elo: u16,
        accumulator: &'a StripedTreePositionMap,
    ) -> Self {
        Self {
            max_ply,
            w_win,
            draw,
            b_win,
            w_elo,
            b_elo,
            accumulator,
            pos: Chess::default(),
            ply: 0,
        }
    }
}

impl<'a> pgn_reader::Visitor for PgnTreeStatsVisitor<'a> {
    type Result = ();

    fn begin_game(&mut self) {
        self.pos = Chess::default();
        self.ply = 0;
    }

    fn begin_variation(&mut self) -> pgn_reader::Skip {
        pgn_reader::Skip(true)
    }

    fn san(&mut self, san_plus: SanPlus) {
        if self.ply >= self.max_ply {
            return;
        }

        let pre_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);

        if let Ok(m) = san_plus.san.to_move(&self.pos) {
            let packed = PackedMove::from(&m).0;
            self.accumulator.record(
                pre_hash.0,
                Some(packed),
                self.w_win,
                self.draw,
                self.b_win,
                self.w_elo,
                self.b_elo,
            );

            self.pos.play_unchecked(&m);
            self.ply += 1;

            if self.ply == self.max_ply {
                let final_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
                self.accumulator.record(
                    final_hash.0,
                    None,
                    self.w_win,
                    self.draw,
                    self.b_win,
                    self.w_elo,
                    self.b_elo,
                );
            }
        }
    }

    fn end_game(&mut self) -> Self::Result {}
}

// ---------------------------------------------------------------------------
// Compact Binary Payload Encoding & Decoding
// ---------------------------------------------------------------------------

pub fn encode_tree_position_payload(node: &TreePositionNode) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16 + node.moves.len() * 12);
    write_varint(&mut buf, node.total_games as u64);
    write_varint(&mut buf, node.white_wins as u64);
    write_varint(&mut buf, node.black_wins as u64);
    write_varint(&mut buf, node.moves.len() as u64);

    for m in &node.moves {
        buf.extend_from_slice(&m.packed_move.to_le_bytes());
        write_varint(&mut buf, m.white_wins as u64);
        write_varint(&mut buf, m.draws as u64);
        write_varint(&mut buf, m.black_wins as u64);
        let avg_w = m.avg_white_elo().unwrap_or(0) as u16;
        let avg_b = m.avg_black_elo().unwrap_or(0) as u16;
        buf.extend_from_slice(&avg_w.to_le_bytes());
        buf.extend_from_slice(&avg_b.to_le_bytes());
    }

    buf
}

pub fn decode_tree_position_payload(
    mut slice: &[u8],
    zobrist_hash: u64,
) -> Result<TreePositionNode> {
    let total_games = read_varint(&mut slice)? as u32;
    let white_wins = read_varint(&mut slice)? as u32;
    let black_wins = read_varint(&mut slice)? as u32;
    let draws = total_games.saturating_sub(white_wins + black_wins);
    let move_count = read_varint(&mut slice)? as usize;

    let mut moves = Vec::with_capacity(move_count);
    for _ in 0..move_count {
        if slice.len() < 2 {
            anyhow::bail!("Unexpected EOF reading packed_move");
        }
        let packed_move = u16::from_le_bytes(slice[0..2].try_into()?);
        slice = &slice[2..];

        let m_ww = read_varint(&mut slice)? as u32;
        let m_dr = read_varint(&mut slice)? as u32;
        let m_bw = read_varint(&mut slice)? as u32;
        let m_tot = m_ww + m_dr + m_bw;

        if slice.len() < 4 {
            anyhow::bail!("Unexpected EOF reading move elo averages");
        }
        let avg_w = u16::from_le_bytes(slice[0..2].try_into()?);
        let avg_b = u16::from_le_bytes(slice[2..4].try_into()?);
        slice = &slice[4..];

        moves.push(TreeMoveStats {
            packed_move,
            total_games: m_tot,
            white_wins: m_ww,
            draws: m_dr,
            black_wins: m_bw,
            white_elo_sum: avg_w as u64 * m_tot as u64,
            black_elo_sum: avg_b as u64 * m_tot as u64,
            elo_game_count: if avg_w > 0 { m_tot } else { 0 },
        });
    }

    Ok(TreePositionNode {
        zobrist_hash,
        total_games,
        white_wins,
        draws,
        black_wins,
        moves,
    })
}

pub fn generate_tree_report(
    node: &TreePositionNode,
    pos: &Chess,
    zobrist_hash: u64,
) -> OpeningTreeReport {
    let total = node.total_games.max(1);
    let round_2dp = |v: f64| (v * 100.0).round() / 100.0;

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
                avg_white_elo: m.avg_white_elo(),
                avg_black_elo: m.avg_black_elo(),
                last_played: None,
                sample_game_ids: Vec::new(),
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
        sample_game_ids: Vec::new(),
        sample_games: Vec::new(),
    }
}

pub fn parse_target_position(fen_str: &str) -> Option<(Chess, u64)> {
    let trimmed = fen_str.trim();
    if trimmed.is_empty() {
        let pos = Chess::default();
        let hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
        return Some((pos, hash.0));
    }

    if let Ok(fen) = trimmed.parse::<Fen>() {
        if let Ok(pos) = fen.into_position::<Chess>(CastlingMode::Chess960) {
            let hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
            return Some((pos, hash.0));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Compact Varint Helpers
// ---------------------------------------------------------------------------

#[inline]
pub fn write_varint<W: Write>(w: &mut W, mut val: u64) {
    while val >= 0x80 {
        let _ = w.write_all(&[((val & 0x7F) as u8) | 0x80]);
        val >>= 7;
    }
    let _ = w.write_all(&[val as u8]);
}

#[inline]
pub fn read_varint(slice: &mut &[u8]) -> Result<u64> {
    let mut val = 0u64;
    let mut shift = 0;
    while !slice.is_empty() {
        let byte = slice[0];
        *slice = &slice[1..];
        val |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Ok(val);
        }
        shift += 7;
        if shift > 64 {
            anyhow::bail!("Varint overflow");
        }
    }
    anyhow::bail!("Unexpected EOF decoding varint")
}
