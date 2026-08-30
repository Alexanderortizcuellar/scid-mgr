use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Position};

pub const POS_INDEX_MAGIC: &[u8; 8] = b"SCIDPOS1";
pub const DEFAULT_MAX_PLY_DEPTH: usize = 24; // 12 full moves

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
    pub total_games: usize,
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
    pub white_elo_sum: u64,
    pub black_elo_sum: u64,
    pub elo_count: usize,
    pub sample_game_ids: Vec<usize>, // top 20 sample game IDs for instant preview
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionNode {
    pub zobrist_hash: u64,
    pub total_games: usize,
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
    pub moves: HashMap<String, MoveStats>, // Key is UCI string (e.g. "e2e4")
    pub game_ids: Vec<usize>,             // list of all game IDs reaching this position
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTreeMoveView {
    pub san: String,
    pub uci: String,
    pub total_games: usize,
    pub white_pct: f64,
    pub draw_pct: f64,
    pub black_pct: f64,
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
    pub avg_white_elo: Option<u32>,
    pub avg_black_elo: Option<u32>,
    pub sample_game_ids: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTreeReport {
    pub fen: String,
    pub zobrist_hash: u64,
    pub total_games: usize,
    pub white_wins: usize,
    pub draws: usize,
    pub black_wins: usize,
    pub white_pct: f64,
    pub draw_pct: f64,
    pub black_pct: f64,
    pub moves: Vec<OpeningTreeMoveView>,
    pub game_ids: Vec<usize>,
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

impl PositionIndex {
    pub fn companion_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
        let p = db_path.as_ref();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.is_empty() {
            p.with_extension("pos.idx")
        } else {
            let file_name = p.file_name().unwrap_or_default().to_string_lossy();
            p.with_file_name(format!("{}.pos.idx", file_name))
        }
    }

    pub fn check_status<P: AsRef<Path>>(
        db_path: P,
        expected_game_count: usize,
    ) -> (IndexStatus, Option<PositionIndexHeader>) {
        let idx_path = Self::companion_path(&db_path);
        if !idx_path.exists() {
            return (IndexStatus::Missing, None);
        }

        let file = match File::open(&idx_path) {
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

        let db_metadata = match std::fs::metadata(&db_path) {
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
        let idx_path = Self::companion_path(&db_path);
        let file = File::open(&idx_path)
            .with_context(|| format!("Failed to open position index: {}", idx_path.display()))?;
        let mut reader = BufReader::new(file);
        let data: PositionIndexData = bincode::deserialize_from(&mut reader)
            .with_context(|| "Failed to deserialize position index data")?;

        Ok(Self {
            path: idx_path,
            data,
        })
    }

    pub fn save<P: AsRef<Path>>(db_path: P, data: &PositionIndexData) -> Result<PathBuf> {
        let idx_path = Self::companion_path(&db_path);
        let temp_path = idx_path.with_extension("tmp");
        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);

        bincode::serialize_into(&mut writer, data)?;
        std::mem::drop(writer);

        if idx_path.exists() {
            let _ = std::fs::remove_file(&idx_path);
        }
        std::fs::rename(&temp_path, &idx_path)?;

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
            .values()
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
            game_ids: node.game_ids.clone(),
        })
    }

    /// Fast lookup of matching game IDs for position search (< 1ms)
    pub fn get_position_game_ids(&self, zobrist_hash: u64) -> Option<&[usize]> {
        self.data
            .positions
            .get(&zobrist_hash)
            .map(|node| node.game_ids.as_slice())
    }

    /// Build companion .pos.idx for SCID databases in parallel
    pub fn build_for_scid<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: &Path,
        max_ply: usize,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let total_games = entries.len();
        let file = File::open(games_path)
            .with_context(|| format!("Failed to open games file: {}", games_path.display()))?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let chunk_size = 2000;
        let scanned_counter = AtomicUsize::new(0);

        let positions_map: HashMap<u64, PositionNode> = (0..total_games)
            .into_par_iter()
            .step_by(chunk_size)
            .map(|start_idx| {
                let end_idx = (start_idx + chunk_size).min(total_games);
                let mut local_map: HashMap<u64, PositionNode> = HashMap::new();

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

                        record_step(
                            &mut local_map,
                            current_hash.0,
                            san.to_string(),
                            uci,
                            w_win,
                            draw,
                            b_win,
                            entry.white_elo,
                            entry.black_elo,
                            game_id,
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

                let current_scanned = scanned_counter.fetch_add(end_idx - start_idx, Ordering::Relaxed) + (end_idx - start_idx);
                progress(current_scanned, total_games, local_map.len());

                local_map
            })
            .reduce(HashMap::new, merge_maps);

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

    /// Build companion .pos.idx for PGN databases in parallel
    pub fn build_for_pgn<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        max_ply: usize,
        progress: F,
    ) -> Result<Self> {
        let db_p = db_path.as_ref();
        let total_games = entries.len();
        let chunk_size = 2000;
        let scanned_counter = AtomicUsize::new(0);

        let positions_map: HashMap<u64, PositionNode> = (0..total_games)
            .into_par_iter()
            .step_by(chunk_size)
            .map(|start_idx| {
                let end_idx = (start_idx + chunk_size).min(total_games);
                let mut local_map: HashMap<u64, PositionNode> = HashMap::new();

                for game_id in start_idx..end_idx {
                    let entry = &entries[game_id];
                    let (w_win, draw, b_win) = match entry.result.as_str() {
                        "1-0" => (1, 0, 0),
                        "0-1" => (0, 0, 1),
                        "1/2-1/2" => (0, 1, 0),
                        _ => (0, 0, 0),
                    };

                    let slice = &mmap[entry.offset as usize..(entry.offset as usize + entry.length as usize)];
                    let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                    let mut indexer = PgnTreeIndexer::new(
                        max_ply,
                        w_win,
                        draw,
                        b_win,
                        entry.white_elo.unwrap_or(0),
                        entry.black_elo.unwrap_or(0),
                        game_id,
                    );
                    let _ = reader.read_game(&mut indexer);
                    for (h, node) in indexer.local_map {
                        let existing = local_map.entry(h).or_insert_with(|| PositionNode {
                            zobrist_hash: h,
                            total_games: 0,
                            white_wins: 0,
                            draws: 0,
                            black_wins: 0,
                            moves: HashMap::new(),
                            game_ids: Vec::new(),
                        });
                        existing.total_games += node.total_games;
                        existing.white_wins += node.white_wins;
                        existing.draws += node.draws;
                        existing.black_wins += node.black_wins;
                        existing.game_ids.extend(node.game_ids);
                        for (u, mv) in node.moves {
                            let ex_mv = existing.moves.entry(u.clone()).or_insert_with(|| MoveStats {
                                san: mv.san.clone(),
                                uci: u,
                                total_games: 0,
                                white_wins: 0,
                                draws: 0,
                                black_wins: 0,
                                white_elo_sum: 0,
                                black_elo_sum: 0,
                                elo_count: 0,
                                sample_game_ids: Vec::new(),
                            });
                            ex_mv.total_games += mv.total_games;
                            ex_mv.white_wins += mv.white_wins;
                            ex_mv.draws += mv.draws;
                            ex_mv.black_wins += mv.black_wins;
                            ex_mv.white_elo_sum += mv.white_elo_sum;
                            ex_mv.black_elo_sum += mv.black_elo_sum;
                            ex_mv.elo_count += mv.elo_count;
                            for id in mv.sample_game_ids {
                                if ex_mv.sample_game_ids.len() < 20 {
                                    ex_mv.sample_game_ids.push(id);
                                }
                            }
                        }
                    }
                }

                let current_scanned = scanned_counter.fetch_add(end_idx - start_idx, Ordering::Relaxed) + (end_idx - start_idx);
                progress(current_scanned, total_games, local_map.len());

                local_map
            })
            .reduce(HashMap::new, merge_maps);

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

fn record_step(
    positions: &mut HashMap<u64, PositionNode>,
    zobrist: u64,
    san: String,
    uci: String,
    w_win: usize,
    draw: usize,
    b_win: usize,
    white_elo: u16,
    black_elo: u16,
    game_id: usize,
) {
    let node = positions.entry(zobrist).or_insert_with(|| PositionNode {
        zobrist_hash: zobrist,
        total_games: 0,
        white_wins: 0,
        draws: 0,
        black_wins: 0,
        moves: HashMap::new(),
        game_ids: Vec::new(),
    });

    node.total_games += 1;
    node.white_wins += w_win;
    node.draws += draw;
    node.black_wins += b_win;
    if node.game_ids.last().copied() != Some(game_id) {
        node.game_ids.push(game_id);
    }

    let move_stat = node.moves.entry(uci.clone()).or_insert_with(|| MoveStats {
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

    move_stat.total_games += 1;
    move_stat.white_wins += w_win;
    move_stat.draws += draw;
    move_stat.black_wins += b_win;
    if white_elo > 0 && black_elo > 0 {
        move_stat.white_elo_sum += white_elo as u64;
        move_stat.black_elo_sum += black_elo as u64;
        move_stat.elo_count += 1;
    }
    if move_stat.sample_game_ids.len() < 20 {
        move_stat.sample_game_ids.push(game_id);
    }
}

fn merge_maps(
    mut a: HashMap<u64, PositionNode>,
    b: HashMap<u64, PositionNode>,
) -> HashMap<u64, PositionNode> {
    for (hash, b_node) in b {
        let a_node = a.entry(hash).or_insert_with(|| PositionNode {
            zobrist_hash: hash,
            total_games: 0,
            white_wins: 0,
            draws: 0,
            black_wins: 0,
            moves: HashMap::new(),
            game_ids: Vec::new(),
        });
        a_node.total_games += b_node.total_games;
        a_node.white_wins += b_node.white_wins;
        a_node.draws += b_node.draws;
        a_node.black_wins += b_node.black_wins;
        a_node.game_ids.extend(b_node.game_ids);

        for (uci, b_mv) in b_node.moves {
            let a_mv = a_node.moves.entry(uci.clone()).or_insert_with(|| MoveStats {
                san: b_mv.san.clone(),
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
            a_mv.total_games += b_mv.total_games;
            a_mv.white_wins += b_mv.white_wins;
            a_mv.draws += b_mv.draws;
            a_mv.black_wins += b_mv.black_wins;
            a_mv.white_elo_sum += b_mv.white_elo_sum;
            a_mv.black_elo_sum += b_mv.black_elo_sum;
            a_mv.elo_count += b_mv.elo_count;
            for id in b_mv.sample_game_ids {
                if a_mv.sample_game_ids.len() < 20 {
                    a_mv.sample_game_ids.push(id);
                }
            }
        }
    }
    a
}

struct PgnTreeIndexer {
    max_ply: usize,
    ply: usize,
    w_win: usize,
    draw: usize,
    b_win: usize,
    white_elo: u16,
    black_elo: u16,
    game_id: usize,
    pos: Chess,
    local_map: HashMap<u64, PositionNode>,
}

impl PgnTreeIndexer {
    fn new(
        max_ply: usize,
        w_win: usize,
        draw: usize,
        b_win: usize,
        white_elo: u16,
        black_elo: u16,
        game_id: usize,
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
            local_map: HashMap::new(),
        }
    }
}

impl pgn_reader::Visitor for PgnTreeIndexer {
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

            record_step(
                &mut self.local_map,
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
