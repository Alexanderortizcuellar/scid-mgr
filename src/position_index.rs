use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
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

pub const POS_INDEX_MAGIC: &[u8; 8] = b"SCIDPOS1";
pub const DEFAULT_MAX_PLY_DEPTH: usize = 16; // 8 full moves (covers standard opening repertoire)
const NUM_STRIPES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndexStatus {
    Valid,
    Outdated,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionIndexHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub db_mtime_secs: u64,
    pub db_size_bytes: u64,
    pub db_game_count: usize,
    pub max_ply_depth: usize,
    pub unique_positions: usize,
    pub created_timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoveStats {
    pub san: String,
    pub uci: String,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub white_elo_sum: u64,
    pub black_elo_sum: u64,
    pub elo_count: u32,
    pub sample_game_ids: Vec<u32>, // Capped at max 20 game IDs
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionNode {
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub moves: Vec<MoveStats>,     // Compact contiguous list
    pub sample_game_ids: Vec<u32>, // Capped at max 50 game IDs
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionIndexData {
    pub header: PositionIndexHeader,
    pub positions: HashMap<u64, PositionNode>,
}

pub struct PositionIndex {
    pub path: PathBuf,
    pub data: PositionIndexData,
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
        san: String,
        uci: String,
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

        let move_stat = if let Some(pos) = node.moves.iter().position(|m| m.uci == uci) {
            &mut node.moves[pos]
        } else {
            node.moves.push(MoveStats {
                san,
                uci,
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

impl PositionIndex {
    /// Determines the companion .pos.idx path for any database file
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

    /// Checks if a companion .pos.idx exists and is valid for the given database
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

        let mut reader = BufReader::new(file);
        let header: PositionIndexHeader = match bincode::deserialize_from(&mut reader) {
            Ok(h) => h,
            Err(_) => return (IndexStatus::Outdated, None),
        };

        if &header.magic != POS_INDEX_MAGIC {
            return (IndexStatus::Outdated, Some(header));
        }

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

        // Check if database game count or modification timestamp mismatch
        if header.db_game_count != expected_game_count || header.db_mtime_secs != current_mtime {
            return (IndexStatus::Outdated, Some(header));
        }

        (IndexStatus::Valid, Some(header))
    }

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
                return Err(anyhow::anyhow!("Position index not found: {}", idx_path.display()));
            }
        };

        let file = File::open(&actual_path)
            .with_context(|| format!("Failed to open position index: {}", actual_path.display()))?;
        let mut reader = BufReader::with_capacity(1024 * 1024, file);
        let data: PositionIndexData = bincode::deserialize_from(&mut reader)
            .with_context(|| "Failed to deserialize position index data")?;

        Ok(Self {
            path: actual_path,
            data,
        })
    }

    pub fn save<P: AsRef<Path>>(db_path: P, data: &PositionIndexData) -> Result<PathBuf> {
        let idx_path = Self::companion_path(&db_path);
        let temp_path = idx_path.with_file_name(format!(
            "{}.tmp",
            idx_path.file_name().unwrap_or_default().to_string_lossy()
        ));

        {
            let file = File::create(&temp_path)
                .with_context(|| format!("Failed to create temporary index file: {}", temp_path.display()))?;
            let mut writer = BufWriter::with_capacity(1024 * 1024, file);
            bincode::serialize_into(&mut writer, data)
                .with_context(|| "Failed to serialize position index data")?;
            writer.flush()?;
        }

        if idx_path.exists() {
            let _ = std::fs::remove_file(&idx_path);
        }
        std::fs::rename(&temp_path, &idx_path)
            .with_context(|| format!("Failed to rename {} to {}", temp_path.display(), idx_path.display()))?;

        eprintln!(
            "[PositionIndex] Successfully saved index: {} ({} unique positions, {:.2} MB)",
            idx_path.display(),
            data.positions.len(),
            std::fs::metadata(&idx_path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0)
        );

        Ok(idx_path)
    }

    /// Query the opening tree for any board position (FEN or standard starting board)
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

        let node = self.data.positions.get(&zobrist_hash)?;
        let total = node.total_games.max(1);

        let mut move_views: Vec<OpeningTreeMoveView> = node
            .moves
            .iter()
            .map(|m| {
                let m_total = m.total_games.max(1);
                OpeningTreeMoveView {
                    san: m.san.clone(),
                    uci: m.uci.clone(),
                    total_games: m.total_games,
                    white_pct: (m.white_wins as f64 / m_total as f64) * 100.0,
                    draw_pct: (m.draws as f64 / m_total as f64) * 100.0,
                    black_pct: (m.black_wins as f64 / m_total as f64) * 100.0,
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

        Some(OpeningTreeReport {
            fen: format!("{:?}", pos),
            zobrist_hash,
            total_games: node.total_games,
            white_wins: node.white_wins,
            draws: node.draws,
            black_wins: node.black_wins,
            white_pct: (node.white_wins as f64 / total as f64) * 100.0,
            draw_pct: (node.draws as f64 / total as f64) * 100.0,
            black_pct: (node.black_wins as f64 / total as f64) * 100.0,
            moves: move_views,
            sample_game_ids: node.sample_game_ids.clone(),
        })
    }

    /// Fast lookup of sample matching game IDs for position search (< 1ms)
    pub fn get_position_sample_games(&self, zobrist_hash: u64) -> Option<&[u32]> {
        self.data
            .positions
            .get(&zobrist_hash)
            .map(|node| node.sample_game_ids.as_slice())
    }

    /// Build companion .pos.idx for SCID databases in parallel with configurable threads and striped lock accumulator
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

                        // Flags check
                        let mut cursor = 0;
                        while cursor < blob.len() && blob[cursor] != 0 {
                            let tag_len = blob[cursor] as usize;
                            cursor += 1 + tag_len;
                        }
                        if cursor < blob.len() && blob[cursor] == 0 {
                            cursor += 1;
                        }
                        if cursor >= blob.len() {
                            continue;
                        }

                        let flags = blob[cursor];
                        cursor += 1;

                        let mut pos = if flags & 0x01 != 0 {
                            let fen_start = cursor;
                            while cursor < blob.len() && blob[cursor] != 0 {
                                cursor += 1;
                            }
                            let fen_bytes = &blob[fen_start..cursor];
                            cursor += 1;
                            if let Ok(fen_str) = std::str::from_utf8(fen_bytes) {
                                if let Ok(fen) = fen_str.parse::<Fen>() {
                                    if let Ok(p) = fen.into_position(CastlingMode::Standard) {
                                        p
                                    } else {
                                        Chess::default()
                                    }
                                } else {
                                    Chess::default()
                                }
                            } else {
                                Chess::default()
                            }
                        } else {
                            Chess::default()
                        };

                        let mut slots = crate::position_search::standard_piece_slots();
                        let mut counts = [16usize, 16];
                        let mut ply = 0;

                        // Step through move stream
                        while cursor < blob.len() && ply < max_ply {
                            let byte = blob[cursor];
                            cursor += 1;

                            if byte == 15 {
                                // ENCODE_END_GAME
                                break;
                            }
                            if byte == 11 {
                                cursor += 1;
                                continue;
                            }
                            if byte == 12 {
                                continue;
                            }
                            if byte == 13 || byte == 14 {
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
                            let mut pos_clone = pos.clone();
                            let san = SanPlus::from_move_and_play_unchecked(&mut pos_clone, &mv);
                            let uci = mv.to_uci(CastlingMode::Standard).to_string();

                            accumulator.record_step(
                                current_hash.0,
                                san.to_string(),
                                uci,
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

                            pos.play_unchecked(&mv);
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

        let positions_map = accumulator.into_map();

        let db_metadata = std::fs::metadata(db_p)?;
        let mtime_secs = db_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let header = PositionIndexHeader {
            magic: *POS_INDEX_MAGIC,
            version: 1,
            db_mtime_secs: mtime_secs,
            db_size_bytes: db_metadata.len(),
            db_game_count: total_games,
            max_ply_depth: max_ply,
            unique_positions: positions_map.len(),
            created_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let data = PositionIndexData {
            header,
            positions: positions_map,
        };

        let idx_path = Self::save(db_p, &data)?;
        Ok(Self {
            path: idx_path,
            data,
        })
    }

    /// Build companion .pos.idx for PGN databases in parallel with configurable threads and striped lock accumulator
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

        let positions_map = accumulator.into_map();

        let db_metadata = std::fs::metadata(db_p)?;
        let mtime_secs = db_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let header = PositionIndexHeader {
            magic: *POS_INDEX_MAGIC,
            version: 1,
            db_mtime_secs: mtime_secs,
            db_size_bytes: db_metadata.len(),
            db_game_count: total_games,
            max_ply_depth: max_ply,
            unique_positions: positions_map.len(),
            created_timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let data = PositionIndexData {
            header,
            positions: positions_map,
        };

        let idx_path = Self::save(db_p, &data)?;
        Ok(Self {
            path: idx_path,
            data,
        })
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
            let uci = m.to_uci(CastlingMode::Standard).to_string();
            let san_str = san_plus.to_string();

            self.accumulator.record_step(
                current_hash.0,
                san_str,
                uci,
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
