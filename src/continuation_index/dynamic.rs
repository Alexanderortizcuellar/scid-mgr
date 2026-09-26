use memmap2::Mmap;
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rayon::prelude::*;
use shakmaty::fen::Fen;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::Ordering;

use super::codec::{format_continuation_moves, parse_fen_fullmove};
use super::types::{ContinuationLine, ContinuationQuery, ContinuationResult, PackedMove};

#[derive(Debug, Clone, Default)]
struct PathStats {
    games: u64,
    white_wins: u64,
    draws: u64,
    black_wins: u64,
}

pub fn calculate_continuations_for_scid<P: AsRef<Path>>(
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: P,
    query: &ContinuationQuery,
    candidate_game_ids: Option<&[usize]>,
) -> Option<ContinuationResult> {
    let target_pos = query.validate().ok()?;
    let target_hash_val: Zobrist64 = target_pos.zobrist_hash(EnPassantMode::Legal);
    let target_hash = target_hash_val.0;

    let file = File::open(games_path.as_ref()).ok()?;
    let mmap = unsafe { Mmap::map(&file).ok()? };

    let max_depth = query.max_depth;
    let max_scan_ply = 60;

    let process_game = |game_id: usize| -> Option<(bool, Option<Vec<PackedMove>>, u32, u32, u32)> {
        if game_id >= entries.len() {
            return None;
        }
        let entry = &entries[game_id];
        if entry.deleted {
            return None;
        }

        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > mmap.len() || start >= end {
            return None;
        }

        let blob = &mmap[start..end];
        let mut cursor = 0;

        let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
            Some(p) => p,
            None => return None,
        };

        let (w_win, draw, b_win) = match entry.result {
            1 => (1, 0, 0),
            2 => (0, 0, 1),
            3 => (0, 1, 0),
            _ => (0, 0, 0),
        };

        let mut slots = crate::position_search::standard_piece_slots();
        let mut counts = [16usize, 16usize];
        let mut ply = 0;

        let mut reached_target = false;
        let mut continuation_moves: Vec<PackedMove> = Vec::new();

        while cursor < blob.len() && ply < max_scan_ply {
            let curr_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
            if curr_hash.0 == target_hash && !reached_target {
                reached_target = true;
            }

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

            if reached_target && continuation_moves.len() < max_depth {
                continuation_moves.push(PackedMove::from(&mv));
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

            if reached_target && continuation_moves.len() >= max_depth {
                break;
            }
        }

        if reached_target {
            Some((true, Some(continuation_moves), w_win, draw, b_win))
        } else {
            Some((false, None, 0, 0, 0))
        }
    };

    let (games_reaching, continuation_map) = if let Some(cand) = candidate_game_ids {
        cand.par_iter()
            .filter_map(|&gid| process_game(gid))
            .fold(
                || (0u64, HashMap::<Vec<PackedMove>, PathStats>::new()),
                |(mut reached, mut map), (hit, path_opt, w, d, b)| {
                    if hit {
                        reached += 1;
                        if let Some(path) = path_opt {
                            if !path.is_empty() {
                                let entry = map.entry(path).or_default();
                                entry.games += 1;
                                entry.white_wins += w as u64;
                                entry.draws += d as u64;
                                entry.black_wins += b as u64;
                            }
                        }
                    }
                    (reached, map)
                },
            )
            .reduce(
                || (0u64, HashMap::new()),
                |(mut r1, mut m1), (r2, m2)| {
                    r1 += r2;
                    for (k, v) in m2 {
                        let e = m1.entry(k).or_default();
                        e.games += v.games;
                        e.white_wins += v.white_wins;
                        e.draws += v.draws;
                        e.black_wins += v.black_wins;
                    }
                    (r1, m1)
                },
            )
    } else {
        (0..entries.len())
            .into_par_iter()
            .filter_map(process_game)
            .fold(
                || (0u64, HashMap::<Vec<PackedMove>, PathStats>::new()),
                |(mut reached, mut map), (hit, path_opt, w, d, b)| {
                    if hit {
                        reached += 1;
                        if let Some(path) = path_opt {
                            if !path.is_empty() {
                                let entry = map.entry(path).or_default();
                                entry.games += 1;
                                entry.white_wins += w as u64;
                                entry.draws += d as u64;
                                entry.black_wins += b as u64;
                            }
                        }
                    }
                    (reached, map)
                },
            )
            .reduce(
                || (0u64, HashMap::new()),
                |(mut r1, mut m1), (r2, m2)| {
                    r1 += r2;
                    for (k, v) in m2 {
                        let e = m1.entry(k).or_default();
                        e.games += v.games;
                        e.white_wins += v.white_wins;
                        e.draws += v.draws;
                        e.black_wins += v.black_wins;
                    }
                    (r1, m1)
                },
            )
    };

    let total_games = entries.iter().filter(|e| !e.deleted).count() as u64;
    Some(build_final_result(
        query,
        &target_pos,
        total_games,
        games_reaching,
        continuation_map,
    ))
}

pub fn calculate_continuations_for_pgn<P: AsRef<Path>>(
    pgn_path: P,
    query: &ContinuationQuery,
    candidate_game_ids: Option<&[usize]>,
) -> Option<ContinuationResult> {
    let target_pos = query.validate().ok()?;
    let target_hash_val: Zobrist64 = target_pos.zobrist_hash(EnPassantMode::Legal);
    let target_hash = target_hash_val.0;

    let pgn_file = File::open(pgn_path.as_ref()).ok()?;
    let mmap = unsafe { Mmap::map(&pgn_file).ok()? };

    let candidate_set: Option<HashSet<usize>> =
        candidate_game_ids.map(|ids| ids.iter().copied().collect());

    let max_depth = query.max_depth;

    let num_threads = rayon::current_num_threads();
    let chunk_size = (mmap.len() / num_threads).max(64 * 1024);
    let mut chunk_offsets = Vec::new();
    let mut curr = 0;

    while curr < mmap.len() {
        let mut next = (curr + chunk_size).min(mmap.len());
        if next < mmap.len() {
            while next < mmap.len() && mmap[next] != b'\n' {
                next += 1;
            }
            if next < mmap.len() {
                next += 1;
            }
        }
        chunk_offsets.push((curr, next));
        curr = next;
    }

    let global_game_idx = std::sync::atomic::AtomicUsize::new(0);

    let (games_reaching, continuation_map) = chunk_offsets
        .par_iter()
        .map(|&(start, end)| {
            let chunk = &mmap[start..end];
            let mut reader = BufferedReader::new_cursor(chunk);
            let mut reached_local = 0u64;
            let mut map_local: HashMap<Vec<PackedMove>, PathStats> = HashMap::new();

            struct ContinuationPgnVisitor {
                target_hash: u64,
                max_depth: usize,
                pos: Chess,
                reached_target: bool,
                moves: Vec<PackedMove>,
                w_win: u32,
                draw: u32,
                b_win: u32,
            }

            impl Visitor for ContinuationPgnVisitor {
                type Result = (bool, Vec<PackedMove>, u32, u32, u32);

                fn header(&mut self, key: &[u8], value: pgn_reader::RawHeader<'_>) {
                    if key == b"Result" {
                        let val = value.as_bytes();
                        if val == b"1-0" {
                            self.w_win = 1;
                        } else if val == b"0-1" {
                            self.b_win = 1;
                        } else if val == b"1/2-1/2" {
                            self.draw = 1;
                        }
                    } else if key == b"FEN" {
                        if let Ok(fen_str) = std::str::from_utf8(value.as_bytes()) {
                            if let Ok(fen) = fen_str.parse::<Fen>() {
                                if let Ok(custom_pos) = fen.into_position(CastlingMode::Standard) {
                                    self.pos = custom_pos;
                                }
                            }
                        }
                    }
                }

                fn begin_variation(&mut self) -> Skip {
                    Skip(true)
                }

                fn san(&mut self, san: SanPlus) {
                    let curr_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
                    if curr_hash.0 == self.target_hash && !self.reached_target {
                        self.reached_target = true;
                    }

                    if let Ok(m) = san.san.to_move(&self.pos) {
                        if self.reached_target && self.moves.len() < self.max_depth {
                            self.moves.push(PackedMove::from(&m));
                        }
                        self.pos.play_unchecked(&m);
                    }
                }

                fn end_game(&mut self) -> Self::Result {
                    (
                        self.reached_target,
                        std::mem::take(&mut self.moves),
                        self.w_win,
                        self.draw,
                        self.b_win,
                    )
                }
            }

            let mut visitor = ContinuationPgnVisitor {
                target_hash,
                max_depth,
                pos: Chess::default(),
                reached_target: false,
                moves: Vec::new(),
                w_win: 0,
                draw: 0,
                b_win: 0,
            };

            while let Ok(Some((hit, path, w, d, b))) = reader.read_game(&mut visitor) {
                let g_idx = global_game_idx.fetch_add(1, Ordering::Relaxed);

                if let Some(ref cands) = candidate_set {
                    if !cands.contains(&g_idx) {
                        visitor = ContinuationPgnVisitor {
                            target_hash,
                            max_depth,
                            pos: Chess::default(),
                            reached_target: false,
                            moves: Vec::new(),
                            w_win: 0,
                            draw: 0,
                            b_win: 0,
                        };
                        continue;
                    }
                }

                if hit {
                    reached_local += 1;
                    if !path.is_empty() {
                        let e = map_local.entry(path).or_default();
                        e.games += 1;
                        e.white_wins += w as u64;
                        e.draws += d as u64;
                        e.black_wins += b as u64;
                    }
                }

                visitor = ContinuationPgnVisitor {
                    target_hash,
                    max_depth,
                    pos: Chess::default(),
                    reached_target: false,
                    moves: Vec::new(),
                    w_win: 0,
                    draw: 0,
                    b_win: 0,
                };
            }

            (reached_local, map_local)
        })
        .reduce(
            || (0u64, HashMap::new()),
            |(mut r1, mut m1), (r2, m2)| {
                r1 += r2;
                for (k, v) in m2 {
                    let e = m1.entry(k).or_default();
                    e.games += v.games;
                    e.white_wins += v.white_wins;
                    e.draws += v.draws;
                    e.black_wins += v.black_wins;
                }
                (r1, m1)
            },
        );

    let total_games = global_game_idx.load(std::sync::atomic::Ordering::Relaxed) as u64;
    Some(build_final_result(
        query,
        &target_pos,
        total_games,
        games_reaching,
        continuation_map,
    ))
}

fn build_final_result(
    query: &ContinuationQuery,
    start_pos: &Chess,
    total_games_processed: u64,
    games_reaching_position: u64,
    continuation_map: HashMap<Vec<PackedMove>, PathStats>,
) -> ContinuationResult {
    let start_fullmove = parse_fen_fullmove(&query.position);
    let mut final_lines = Vec::new();

    if games_reaching_position > 0 {
        for (packed_path, stats) in continuation_map {
            let percentage = (stats.games as f64 / games_reaching_position as f64) * 100.0;
            if stats.games >= query.min_games && percentage >= query.min_percentage {
                let mut sim_pos = start_pos.clone();
                let mut san_moves = Vec::with_capacity(packed_path.len());

                for pm in packed_path {
                    if let Some(m) = pm.to_shakmaty_move(&sim_pos) {
                        let san_plus = SanPlus::from_move_and_play_unchecked(&mut sim_pos, &m);
                        san_moves.push(san_plus.to_string());
                    } else {
                        san_moves.push(pm.to_uci_string());
                    }
                }

                let formatted = format_continuation_moves(start_pos, start_fullmove, &san_moves);
                final_lines.push(ContinuationLine {
                    moves: san_moves,
                    formatted,
                    games: stats.games,
                    percentage,
                    white_wins: stats.white_wins,
                    draws: stats.draws,
                    black_wins: stats.black_wins,
                });
            }
        }

        final_lines.sort_by(|a, b| {
            b.games
                .cmp(&a.games)
                .then_with(|| b.moves.len().cmp(&a.moves.len()))
                .then_with(|| a.moves.cmp(&b.moves))
        });

        if final_lines.len() > query.max_lines {
            final_lines.truncate(query.max_lines);
        }
    }

    ContinuationResult {
        starting_fen: Fen::from_position(start_pos.clone(), EnPassantMode::Legal).to_string(),
        total_games_processed,
        games_reaching_position,
        lines: final_lines,
        tree: None,
    }
}
