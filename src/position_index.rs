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

pub const POS_INDEX_MAGIC: &[u8; 8] = b"SCIDPOS4";
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

        for m in pos.legal_moves() {
            if m.from() == Some(from_sq) && m.to() == to_sq && m.promotion() == promo {
                return Some(m);
            }
        }
        None
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
#[repr(C)]
pub struct SortedIndexEntry {
    pub hash: u64,
    pub data_offset: u32,
    pub data_len: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoveStats {
    pub packed_move: u16,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub white_elo_sum: u64,
    pub black_elo_sum: u64,
    pub elo_count: u32,
    pub sample_game_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionNode {
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub moves: Vec<MoveStats>,
    pub sample_game_ids: Vec<u32>,
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

        if header.db_game_count != expected_game_count as u64 || header.db_mtime_secs != current_mtime {
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
        let entries = self.index_entries();
        let idx = entries.binary_search_by_key(&target_hash, |e| e.hash).ok()?;
        let entry = &entries[idx];

        let start = (self.header.data_offset + entry.data_offset as u64) as usize;
        let end = start + entry.data_len as usize;
        if end > self.mmap.len() || start >= end {
            return None;
        }

        let payload = &self.mmap[start..end];
        decode_position_payload(payload, target_hash).ok()
    }

    /// Query the opening tree for any board position (FEN or standard starting board) with zero heap RAM overhead
    pub fn query_tree(&self, fen_str: &str) -> Option<OpeningTreeReport> {
        let trimmed = fen_str.trim();
        let (pos, zobrist_hash) = if trimmed.is_empty() {
            let p = Chess::default();
            let h: Zobrist64 = p.zobrist_hash(EnPassantMode::Legal);
            (p, h.0)
        } else if let Ok(fen) = trimmed.parse::<Fen>() {
            if let Ok(p) = fen.into_position::<Chess>(CastlingMode::Standard) {
                let h: Zobrist64 = p.zobrist_hash(EnPassantMode::Legal);
                (p, h.0)
            } else {
                return None;
            }
        } else {
            return None;
        };

        let node = self.get_position(zobrist_hash)?;
        let total = node.total_games.max(1);
        let round_2dp = |v: f64| (v * 100.0).round() / 100.0;

        let mut move_views: Vec<OpeningTreeMoveView> = node
            .moves
            .iter()
            .map(|m| {
                let m_total = m.total_games.max(1);
                let packed = PackedMove(m.packed_move);
                let uci = packed.to_uci_string();
                let san = if let Some(shak_move) = packed.to_shakmaty_move(&pos) {
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
                    avg_white_elo: if m.elo_count > 0 {
                        Some((m.white_elo_sum / m.elo_count as u64) as u32)
                    } else {
                        None
                    },
                    avg_black_elo: if m.elo_count > 0 {
                        Some((m.black_elo_sum / m.elo_count as u64) as u32)
                    } else {
                        None
                    },
                    sample_game_ids: m.sample_game_ids.clone(),
                }
            })
            .collect();

        // Sort moves by popularity (total games played)
        move_views.sort_unstable_by(|a, b| b.total_games.cmp(&a.total_games));

        let fen_formatted = Fen::from_position(pos, EnPassantMode::Legal).to_string();

        Some(OpeningTreeReport {
            fen: fen_formatted,
            zobrist_hash,
            total_games: node.total_games,
            white_wins: node.white_wins,
            draws: node.draws,
            black_wins: node.black_wins,
            white_pct: round_2dp((node.white_wins as f64 / total as f64) * 100.0),
            draw_pct: round_2dp((node.draws as f64 / total as f64) * 100.0),
            black_pct: round_2dp((node.black_wins as f64 / total as f64) * 100.0),
            moves: move_views,
            sample_game_ids: node.sample_game_ids,
        })
    }

    /// Fast lookup of sample matching game IDs for position search (< 0.001 ms)
    pub fn get_position_sample_games(&self, zobrist_hash: u64) -> Option<Vec<u32>> {
        self.get_position(zobrist_hash).map(|n| n.sample_game_ids)
    }

    /// Build static, disk-backed .pos.idx file for SCID databases in parallel
    pub fn build_for_scid<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: &Path,
        max_ply: usize,
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
                                entry.white_elo,
                                entry.black_elo,
                                game_id as u32,
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
        )?;

        Self::load(db_p)
    }

    /// Build static, disk-backed .pos.idx file for PGN databases in parallel
    pub fn build_for_pgn<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        max_ply: usize,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let total_games = entries.len();
        let chunk_size = 5000;
        let scanned_counter = AtomicUsize::new(0);
        let accumulator = StripedPositionMap::new();

        let run_index = || {
            (0..total_games)
                .into_par_iter()
                .step_by(chunk_size)
                .for_each(|start_idx| {
                    let end_idx = (start_idx + chunk_size).min(total_games);

                    for game_id in start_idx..end_idx {
                        let entry = &entries[game_id];
                        let (w_win, draw, b_win) = match entry.result.as_str() {
                            "1-0" => (1, 0, 0),
                            "0-1" => (0, 0, 1),
                            "1/2-1/2" => (0, 1, 0),
                            _ => (0, 0, 0),
                        };

                        let slice =
                            &mmap[entry.offset as usize..(entry.offset as usize + entry.length as usize)];
                        let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                        let mut indexer = PgnTreeVisitor::new(
                            max_ply,
                            w_win,
                            draw,
                            b_win,
                            entry.white_elo.unwrap_or(0),
                            entry.black_elo.unwrap_or(0),
                            game_id as u32,
                            &accumulator,
                        );
                        let _ = reader.read_game(&mut indexer);
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
    ) -> Result<PathBuf> {
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

        for hash in hashes {
            if let Some(node) = positions_map.remove(&hash) {
                let curr_offset = data_payload.len() as u32;
                let payload_bytes = encode_position_payload(&node);
                let curr_len = payload_bytes.len() as u32;
                data_payload.extend_from_slice(&payload_bytes);

                index_entries.push(SortedIndexEntry {
                    hash,
                    data_offset: curr_offset,
                    data_len: curr_len,
                });
            }
        }

        let header = PositionIndexHeader {
            magic: *POS_INDEX_MAGIC,
            version: 2,
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
                writer.write_all(&entry.data_len.to_le_bytes())?;
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

        Ok(idx_path)
    }
}

fn encode_position_payload(node: &PositionNode) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32 + node.moves.len() * 36);
    buf.extend_from_slice(&node.total_games.to_le_bytes());
    buf.extend_from_slice(&node.white_wins.to_le_bytes());
    buf.extend_from_slice(&node.black_wins.to_le_bytes());

    let sample_count = node.sample_game_ids.len().min(50) as u8;
    buf.push(sample_count);
    for &id in &node.sample_game_ids[..sample_count as usize] {
        buf.extend_from_slice(&id.to_le_bytes());
    }

    let move_count = node.moves.len().min(255) as u8;
    buf.push(move_count);

    for m in &node.moves[..move_count as usize] {
        buf.extend_from_slice(&m.packed_move.to_le_bytes());
        buf.extend_from_slice(&m.total_games.to_le_bytes());
        buf.extend_from_slice(&m.white_wins.to_le_bytes());
        buf.extend_from_slice(&m.black_wins.to_le_bytes());
        buf.extend_from_slice(&m.white_elo_sum.to_le_bytes());
        buf.extend_from_slice(&m.black_elo_sum.to_le_bytes());
        buf.extend_from_slice(&m.elo_count.to_le_bytes());

        let m_sample_count = m.sample_game_ids.len().min(20) as u8;
        buf.push(m_sample_count);
        for &id in &m.sample_game_ids[..m_sample_count as usize] {
            buf.extend_from_slice(&id.to_le_bytes());
        }
    }

    buf
}

fn decode_position_payload(mut bytes: &[u8], zobrist_hash: u64) -> Result<PositionNode> {
    if bytes.len() < 13 {
        anyhow::bail!("Payload too small");
    }

    let total_games = u32::from_le_bytes(bytes[0..4].try_into()?);
    let white_wins = u32::from_le_bytes(bytes[4..8].try_into()?);
    let black_wins = u32::from_le_bytes(bytes[8..12].try_into()?);
    let draws = total_games.saturating_sub(white_wins + black_wins);
    bytes = &bytes[12..];

    let sample_count = bytes[0] as usize;
    bytes = &bytes[1..];
    if bytes.len() < sample_count * 4 {
        anyhow::bail!("Corrupt sample game IDs in payload");
    }
    let mut sample_game_ids = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let id = u32::from_le_bytes(bytes[i * 4..(i + 1) * 4].try_into()?);
        sample_game_ids.push(id);
    }
    bytes = &bytes[sample_count * 4..];

    if bytes.is_empty() {
        return Ok(PositionNode {
            zobrist_hash,
            total_games,
            white_wins,
            draws,
            black_wins,
            moves: Vec::new(),
            sample_game_ids,
        });
    }

    let move_count = bytes[0] as usize;
    bytes = &bytes[1..];

    let mut moves = Vec::with_capacity(move_count);
    for _ in 0..move_count {
        if bytes.len() < 35 {
            break;
        }
        let packed_move = u16::from_le_bytes(bytes[0..2].try_into()?);
        let m_total = u32::from_le_bytes(bytes[2..6].try_into()?);
        let m_ww = u32::from_le_bytes(bytes[6..10].try_into()?);
        let m_bw = u32::from_le_bytes(bytes[10..14].try_into()?);
        let m_dr = m_total.saturating_sub(m_ww + m_bw);
        let m_welo = u64::from_le_bytes(bytes[14..22].try_into()?);
        let m_belo = u64::from_le_bytes(bytes[22..30].try_into()?);
        let m_elo_cnt = u32::from_le_bytes(bytes[30..34].try_into()?);
        let m_sample_cnt = bytes[34] as usize;
        bytes = &bytes[35..];

        if bytes.len() < m_sample_cnt * 4 {
            break;
        }
        let mut m_sample_ids = Vec::with_capacity(m_sample_cnt);
        for i in 0..m_sample_cnt {
            let id = u32::from_le_bytes(bytes[i * 4..(i + 1) * 4].try_into()?);
            m_sample_ids.push(id);
        }
        bytes = &bytes[m_sample_cnt * 4..];

        moves.push(MoveStats {
            packed_move,
            total_games: m_total,
            white_wins: m_ww,
            draws: m_dr,
            black_wins: m_bw,
            white_elo_sum: m_welo,
            black_elo_sum: m_belo,
            elo_count: m_elo_cnt,
            sample_game_ids: m_sample_ids,
        });
    }

    Ok(PositionNode {
        zobrist_hash,
        total_games,
        white_wins,
        draws,
        black_wins,
        moves,
        sample_game_ids,
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

    fn record_step(
        &self,
        zobrist: u64,
        packed_move: u16,
        w_win: u32,
        draw: u32,
        b_win: u32,
        white_elo: u16,
        black_elo: u16,
        game_id: u32,
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
            sample_game_ids: Vec::new(),
        });

        node.total_games += 1;
        node.white_wins += w_win;
        node.draws += draw;
        node.black_wins += b_win;
        if node.sample_game_ids.len() < 50 && node.sample_game_ids.last().copied() != Some(game_id) {
            node.sample_game_ids.push(game_id);
        }

        let move_stat = if let Some(pos) = node.moves.iter().position(|m| m.packed_move == packed_move) {
            &mut node.moves[pos]
        } else {
            node.moves.push(MoveStats {
                packed_move,
                total_games: 0,
                white_wins: 0,
                draws: 0,
                black_wins: 0,
                white_elo_sum: 0,
                black_elo_sum: 0,
                elo_count: 0,
                sample_game_ids: Vec::new(),
            });
            node.moves.last_mut().unwrap()
        };

        move_stat.total_games += 1;
        move_stat.white_wins += w_win;
        move_stat.draws += draw;
        move_stat.black_wins += b_win;
        if white_elo > 0 && black_elo > 0 {
            move_stat.white_elo_sum += white_elo as u64;
            move_stat.black_elo_sum += black_elo as u64;
            move_stat.elo_count += 1;
        }
        if move_stat.sample_game_ids.len() < 20 && move_stat.sample_game_ids.last().copied() != Some(game_id) {
            move_stat.sample_game_ids.push(game_id);
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

struct PgnTreeVisitor<'a> {
    max_ply: usize,
    ply: usize,
    w_win: u32,
    draw: u32,
    b_win: u32,
    white_elo: u16,
    black_elo: u16,
    game_id: u32,
    pos: Chess,
    accumulator: &'a StripedPositionMap,
}

impl<'a> PgnTreeVisitor<'a> {
    fn new(
        max_ply: usize,
        w_win: u32,
        draw: u32,
        b_win: u32,
        white_elo: u16,
        black_elo: u16,
        game_id: u32,
        accumulator: &'a StripedPositionMap,
    ) -> Self {
        Self {
            max_ply,
            ply: 0,
            w_win,
            draw,
            b_win,
            white_elo,
            black_elo,
            game_id,
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
                self.white_elo,
                self.black_elo,
                self.game_id,
            );

            self.pos.play_unchecked(&m);
            self.ply += 1;
        }
    }

    fn end_game(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_payload() {
        let e2e4 = PackedMove::new(12, 28, None); // e2 (12) -> e4 (28)
        let d2d4 = PackedMove::new(11, 27, None); // d2 (11) -> d4 (27)

        let node = PositionNode {
            zobrist_hash: 0x123456789abcdef0,
            total_games: 100,
            white_wins: 40,
            draws: 30,
            black_wins: 30,
            moves: vec![
                MoveStats {
                    packed_move: e2e4.0,
                    total_games: 60,
                    white_wins: 25,
                    draws: 20,
                    black_wins: 15,
                    white_elo_sum: 120000,
                    black_elo_sum: 119000,
                    elo_count: 50,
                    sample_game_ids: vec![1, 5, 12],
                },
                MoveStats {
                    packed_move: d2d4.0,
                    total_games: 40,
                    white_wins: 15,
                    draws: 10,
                    black_wins: 15,
                    white_elo_sum: 80000,
                    black_elo_sum: 79500,
                    elo_count: 35,
                    sample_game_ids: vec![2, 7],
                },
            ],
            sample_game_ids: vec![1, 2, 5, 7, 12],
        };

        let encoded = encode_position_payload(&node);
        let decoded = decode_position_payload(&encoded, node.zobrist_hash).expect("decode failed");

        assert_eq!(decoded.zobrist_hash, node.zobrist_hash);
        assert_eq!(decoded.total_games, 100);
        assert_eq!(decoded.white_wins, 40);
        assert_eq!(decoded.draws, 30);
        assert_eq!(decoded.black_wins, 30);
        assert_eq!(decoded.sample_game_ids, vec![1, 2, 5, 7, 12]);
        assert_eq!(decoded.moves.len(), 2);
        assert_eq!(decoded.moves[0].packed_move, e2e4.0);
        assert_eq!(PackedMove(decoded.moves[0].packed_move).to_uci_string(), "e2e4");
        assert_eq!(decoded.moves[0].total_games, 60);
        assert_eq!(decoded.moves[0].sample_game_ids, vec![1, 5, 12]);
        assert_eq!(decoded.moves[1].packed_move, d2d4.0);
        assert_eq!(PackedMove(decoded.moves[1].packed_move).to_uci_string(), "d2d4");
        assert_eq!(decoded.moves[1].total_games, 40);
    }

    #[test]
    fn test_sorted_index_binary_search() {
        let entries = vec![
            SortedIndexEntry { hash: 100, data_offset: 0, data_len: 32 },
            SortedIndexEntry { hash: 200, data_offset: 32, data_len: 48 },
            SortedIndexEntry { hash: 300, data_offset: 80, data_len: 40 },
            SortedIndexEntry { hash: 400, data_offset: 120, data_len: 56 },
        ];

        let idx = entries.binary_search_by_key(&200, |e| e.hash);
        assert_eq!(idx, Ok(1));
        assert_eq!(entries[1].data_offset, 32);

        let missing = entries.binary_search_by_key(&250, |e| e.hash);
        assert!(missing.is_err());
    }
}

