use anyhow::Result;
use rayon::prelude::*;
use shakmaty::fen::Fen;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position, Role, Square};
use std::collections::{HashMap, HashSet};

use crate::continuation_index::{
    format_continuation_moves, parse_fen_fullmove, ContinuationLine, ContinuationQuery,
    ContinuationResult,
};
use crate::tree_index::{OpeningTreeMoveView, OpeningTreeReport};

use super::codec::MmapBoostIndex;
use super::types::{BoostGameMeta, BoostMove};

pub const MAX_PACKED_PATH_PLIES: usize = 16;

/// Compact 256-bit integer-packed path for continuation move sequences (up to 16 plies / 8 full moves)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedPath256 {
    pub lo: u128, // Plies 0..8
    pub hi: u128, // Plies 8..16
    pub len: u8,
}

impl PackedPath256 {
    #[inline(always)]
    pub fn from_slice(slice: &[BoostMove]) -> Self {
        let len = slice.len().min(MAX_PACKED_PATH_PLIES);
        let mut lo = 0u128;
        let mut hi = 0u128;
        for (i, m) in slice[..len.min(8)].iter().enumerate() {
            lo |= (m.0 as u128) << (i * 16);
        }
        if len > 8 {
            for (i, m) in slice[8..len].iter().enumerate() {
                hi |= (m.0 as u128) << (i * 16);
            }
        }
        Self {
            lo,
            hi,
            len: len as u8,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = BoostMove> + '_ {
        let len = self.len as usize;
        (0..len).map(move |i| {
            let m = if i < 8 {
                ((self.lo >> (i * 16)) & 0xFFFF) as u16
            } else {
                ((self.hi >> ((i - 8) * 16)) & 0xFFFF) as u16
            };
            BoostMove(m)
        })
    }

    #[inline(always)]
    pub fn to_boost_moves(&self) -> Vec<BoostMove> {
        self.iter().collect()
    }
}

/// Lightweight 64-byte scratchpad board for sub-nanosecond move application
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FastReplayState {
    /// 0 = Empty, 1..6 = White (P, N, B, R, Q, K), 9..14 = Black (P, N, B, R, Q, K)
    pub board: [u8; 64],
    pub white_pieces: u8,
    pub black_pieces: u8,
}

impl Default for FastReplayState {
    fn default() -> Self {
        Self::new()
    }
}

impl FastReplayState {
    pub fn new() -> Self {
        let mut board = [0u8; 64];
        // Standard starting board placement
        // White pieces (Rank 1 & 2)
        board[0] = 4; // Ra1
        board[1] = 2; // Nb1
        board[2] = 3; // Bc1
        board[3] = 5; // Qd1
        board[4] = 6; // Ke1
        board[5] = 3; // Bf1
        board[6] = 2; // Ng1
        board[7] = 4; // Rh1
        board[8..16].fill(1); // Pawns a2..h2

        // Black pieces (Rank 7 & 8)
        board[48..56].fill(9); // Pawns a7..h7
        board[56] = 12; // Ra8
        board[57] = 10; // Nb8
        board[58] = 11; // Bc8
        board[59] = 13; // Qd8
        board[60] = 14; // Ke8
        board[61] = 11; // Bf8
        board[62] = 10; // Ng8
        board[63] = 12; // Rh8

        Self {
            board,
            white_pieces: 16,
            black_pieces: 16,
        }
    }

    #[inline(always)]
    pub fn count_pieces(board: &[u8; 64]) -> (u8, u8) {
        let mut w = 0u8;
        let mut b = 0u8;
        for &sq in board.iter() {
            if sq >= 1 && sq <= 6 {
                w += 1;
            } else if sq >= 9 && sq <= 14 {
                b += 1;
            }
        }
        (w, b)
    }

    #[inline(always)]
    pub fn apply_move(&mut self, m: BoostMove) {
        let from = m.from();
        let to = m.to();
        let flags = m.flags();
        let piece = self.board[from];

        self.board[from] = 0;

        if flags <= 0x1 {
            // Quiet move or double pawn push
            self.board[to] = piece;
        } else if flags == 0x4 {
            // Normal capture
            let captured = self.board[to];
            if captured >= 1 && captured <= 6 {
                self.white_pieces = self.white_pieces.saturating_sub(1);
            } else if captured >= 9 && captured <= 14 {
                self.black_pieces = self.black_pieces.saturating_sub(1);
            }
            self.board[to] = piece;
        } else if flags == 0x2 {
            // King-side Castle (e1->g1 or e8->g8)
            self.board[to] = piece;
            if to == 6 {
                self.board[7] = 0;
                self.board[5] = 4; // Rh1->f1
            } else if to == 62 {
                self.board[63] = 0;
                self.board[61] = 12; // Rh8->f8
            }
        } else if flags == 0x3 {
            // Queen-side Castle (e1->c1 or e8->c8)
            self.board[to] = piece;
            if to == 2 {
                self.board[0] = 0;
                self.board[3] = 4; // Ra1->d1
            } else if to == 58 {
                self.board[56] = 0;
                self.board[59] = 12; // Ra8->d8
            }
        } else if flags == 0x5 {
            // En Passant Capture
            self.board[to] = piece;
            if piece <= 6 {
                // White captured black pawn on rank 5
                if to >= 8 {
                    self.board[to - 8] = 0;
                    self.black_pieces = self.black_pieces.saturating_sub(1);
                }
            } else {
                // Black captured white pawn on rank 4
                if to + 8 < 64 {
                    self.board[to + 8] = 0;
                    self.white_pieces = self.white_pieces.saturating_sub(1);
                }
            }
        } else if flags >= 0x8 {
            // Promotion
            let captured = self.board[to];
            if captured >= 1 && captured <= 6 {
                self.white_pieces = self.white_pieces.saturating_sub(1);
            } else if captured >= 9 && captured <= 14 {
                self.black_pieces = self.black_pieces.saturating_sub(1);
            }
            let color_offset = if piece >= 8 { 8 } else { 0 };
            let promo_piece = match flags & 0x3 {
                0 => 2 + color_offset, // Knight
                1 => 3 + color_offset, // Bishop
                2 => 4 + color_offset, // Rook
                _ => 5 + color_offset, // Queen
            };
            self.board[to] = promo_piece;
        }
    }
}

/// Converts a `shakmaty::Chess` position board into a 64-byte scratchpad array
pub fn chess_to_board_array(pos: &Chess) -> [u8; 64] {
    let mut board = [0u8; 64];
    for sq in Square::ALL {
        if let Some(piece) = pos.board().piece_at(sq) {
            let role_val = match piece.role {
                Role::Pawn => 1,
                Role::Knight => 2,
                Role::Bishop => 3,
                Role::Rook => 4,
                Role::Queen => 5,
                Role::King => 6,
            };
            let color_offset = if piece.color == shakmaty::Color::White {
                0
            } else {
                8
            };
            board[sq as usize] = role_val + color_offset;
        }
    }
    board
}

/// Result match containing matching game ID and ply numbers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoostMatch {
    pub game_id: usize,
    pub matching_plies: Vec<usize>,
}

/// Fast parallel search evaluator scanning games in `.boost.idx`
pub struct BoostSearchEvaluator<'a> {
    pub index: &'a MmapBoostIndex,
}

impl<'a> BoostSearchEvaluator<'a> {
    pub fn new(index: &'a MmapBoostIndex) -> Self {
        Self { index }
    }

    /// Scans games in parallel for board positions reaching a target FEN with optional turn filter and progress streaming
    pub fn search_position_with_progress<F>(
        &self,
        target_fen: &str,
        turn_filter: Option<&str>,
        max_ply: Option<usize>,
        progress: F,
    ) -> Result<Vec<BoostMatch>>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        let fen: Fen = target_fen
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))?;
        let target_pos: Chess = fen
            .into_position(CastlingMode::Chess960)
            .map_err(|e| anyhow::anyhow!("Invalid position: {}", e))?;

        let target_board = chess_to_board_array(&target_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let total_games = self.index.game_count();

        let turn_req: Option<u8> = match turn_filter {
            Some(t) => {
                let t_low = t.trim().to_lowercase();
                if t_low == "w" || t_low == "white" {
                    Some(0)
                } else if t_low == "b" || t_low == "black" {
                    Some(1)
                } else {
                    None
                }
            }
            None => None,
        };

        let progress_counter = std::sync::atomic::AtomicUsize::new(0);
        let match_counter = std::sync::atomic::AtomicUsize::new(0);
        let step = (total_games / 100).clamp(5_000, 50_000);

        let matches: Vec<BoostMatch> = (0..total_games)
            .into_par_iter()
            .filter_map(|gid| {
                let entry = self.index.get_game_entry(gid)?;
                if entry.is_deleted() || entry.is_custom_fen() {
                    let done =
                        progress_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if done % step == 0 || done == total_games {
                        let cur_matches = match_counter.load(std::sync::atomic::Ordering::Relaxed);
                        progress(done, total_games, cur_matches);
                    }
                    return None;
                }
                let moves = self.index.get_game_moves(gid)?;
                let limit = match max_ply {
                    Some(mp) => moves.len().min(mp),
                    None => moves.len(),
                };

                let mut replay = FastReplayState::new();
                let mut matching_plies = Vec::new();

                if replay.board == target_board {
                    if turn_req.is_none() || turn_req == Some(0) {
                        matching_plies.push(0);
                    }
                }

                for (ply_idx, &m) in moves[..limit].iter().enumerate() {
                    replay.apply_move(m);
                    if replay.white_pieces < target_w || replay.black_pieces < target_b {
                        break;
                    }
                    let ply = ply_idx + 1;
                    if replay.board == target_board {
                        let matches_turn = match turn_req {
                            Some(rem) => (ply % 2) as u8 == rem,
                            None => true,
                        };
                        if matches_turn {
                            matching_plies.push(ply);
                        }
                    }
                }

                let done = progress_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                let res = if !matching_plies.is_empty() {
                    match_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    Some(BoostMatch {
                        game_id: gid,
                        matching_plies,
                    })
                } else {
                    None
                };

                if done % step == 0 || done == total_games {
                    let cur_matches = match_counter.load(std::sync::atomic::Ordering::Relaxed);
                    progress(done, total_games, cur_matches);
                }

                res
            })
            .collect();

        Ok(matches)
    }

    /// Scans games in parallel for board positions reaching a target FEN with optional turn filter
    pub fn search_position_with_options(
        &self,
        target_fen: &str,
        turn_filter: Option<&str>,
        max_ply: Option<usize>,
    ) -> Result<Vec<BoostMatch>> {
        self.search_position_with_progress(target_fen, turn_filter, max_ply, |_, _, _| {})
    }

    /// Scans games in parallel for board positions reaching a target FEN
    pub fn search_position(
        &self,
        target_fen: &str,
        max_ply: Option<usize>,
    ) -> Result<Vec<BoostMatch>> {
        self.search_position_with_progress(target_fen, None, max_ply, |_, _, _| {})
    }

    /// Finds all distinct next moves (and their frequencies) directly following the target position
    pub fn find_next_moves(
        &self,
        target_fen: &str,
        max_ply: Option<usize>,
    ) -> Result<Vec<(BoostMove, u32)>> {
        let fen: Fen = target_fen
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))?;
        let target_pos: Chess = fen
            .into_position(CastlingMode::Chess960)
            .map_err(|e| anyhow::anyhow!("Invalid position: {}", e))?;

        let target_board = chess_to_board_array(&target_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let total_games = self.index.game_count();

        let move_counts = (0..total_games)
            .into_par_iter()
            .fold(HashMap::<u16, u32>::default, |mut acc, gid| {
                let _entry = match self.index.get_game_entry(gid) {
                    Some(e) if !e.is_deleted() && !e.is_custom_fen() => e,
                    _ => return acc,
                };
                let moves = match self.index.get_game_moves(gid) {
                    Some(m) => m,
                    None => return acc,
                };
                let limit = match max_ply {
                    Some(mp) => moves.len().min(mp),
                    None => moves.len(),
                };

                let mut replay = FastReplayState::new();
                if replay.board == target_board && !moves.is_empty() {
                    *acc.entry(moves[0].0).or_insert(0) += 1;
                }

                for (ply_idx, &m) in moves[..limit].iter().enumerate() {
                    replay.apply_move(m);
                    if replay.white_pieces < target_w || replay.black_pieces < target_b {
                        break;
                    }
                    if replay.board == target_board && ply_idx + 1 < moves.len() {
                        *acc.entry(moves[ply_idx + 1].0).or_insert(0) += 1;
                    }
                }

                acc
            })
            .reduce(HashMap::default, |mut map1, map2| {
                for (k, v) in map2 {
                    *map1.entry(k).or_insert(0) += v;
                }
                map1
            });

        let mut sorted: Vec<(BoostMove, u32)> = move_counts
            .into_iter()
            .map(|(k, v)| (BoostMove(k), v))
            .collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.1));

        Ok(sorted)
    }

    /// Finds common continuation move lines (and frequencies) up to `depth` plies after the target position
    pub fn find_continuations(
        &self,
        target_fen: &str,
        depth: usize,
        max_ply: Option<usize>,
    ) -> Result<Vec<(Vec<BoostMove>, u32)>> {
        let fen: Fen = target_fen
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))?;
        let target_pos: Chess = fen
            .into_position(CastlingMode::Chess960)
            .map_err(|e| anyhow::anyhow!("Invalid position: {}", e))?;

        let target_board = chess_to_board_array(&target_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let total_games = self.index.game_count();

        let line_counts = (0..total_games)
            .into_par_iter()
            .fold(HashMap::<PackedPath256, u32>::default, |mut acc, gid| {
                let _entry = match self.index.get_game_entry(gid) {
                    Some(e) if !e.is_deleted() && !e.is_custom_fen() => e,
                    _ => return acc,
                };
                let moves = match self.index.get_game_moves(gid) {
                    Some(m) => m,
                    None => return acc,
                };
                let limit = match max_ply {
                    Some(mp) => moves.len().min(mp),
                    None => moves.len(),
                };

                let mut replay = FastReplayState::new();
                if replay.board == target_board && !moves.is_empty() {
                    let end = depth.min(moves.len());
                    let path = PackedPath256::from_slice(&moves[0..end]);
                    *acc.entry(path).or_insert(0) += 1;
                }

                for (ply_idx, &m) in moves[..limit].iter().enumerate() {
                    replay.apply_move(m);
                    if replay.white_pieces < target_w || replay.black_pieces < target_b {
                        break;
                    }
                    if replay.board == target_board && ply_idx + 1 < moves.len() {
                        let start = ply_idx + 1;
                        let end = (start + depth).min(moves.len());
                        let path = PackedPath256::from_slice(&moves[start..end]);
                        *acc.entry(path).or_insert(0) += 1;
                    }
                }

                acc
            })
            .reduce(HashMap::default, |mut map1, map2| {
                for (k, v) in map2 {
                    *map1.entry(k).or_insert(0) += v;
                }
                map1
            });

        let mut sorted: Vec<(Vec<BoostMove>, u32)> = line_counts
            .into_iter()
            .map(|(path, count)| (path.to_boost_moves(), count))
            .collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.1));

        Ok(sorted)
    }

    /// Quickly samples matching game IDs for a target position with fast early exit
    pub fn sample_position_game_ids(&self, target_pos: &Chess, max_samples: usize) -> Vec<u32> {
        let target_board = chess_to_board_array(target_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let total_games = self.index.header.db_game_count as usize;
        let mut samples = Vec::with_capacity(max_samples);

        for gid in 0..total_games {
            if samples.len() >= max_samples {
                break;
            }
            if let Some(moves) = self.index.get_game_moves(gid) {
                let mut replay = FastReplayState::new();
                if replay.board == target_board {
                    samples.push(gid as u32);
                    continue;
                }
                for &m in moves.iter() {
                    replay.apply_move(m);
                    if replay.white_pieces < target_w || replay.black_pieces < target_b {
                        break;
                    }
                    if replay.board == target_board {
                        samples.push(gid as u32);
                        break;
                    }
                }
            }
        }
        samples
    }

    /// Quickly samples matching game IDs for child moves of a target position with fast early exit
    pub fn sample_child_moves_game_ids(
        &self,
        target_pos: &Chess,
        child_moves: &[crate::tree_index::types::PackedMove],
        max_samples_per_move: usize,
    ) -> HashMap<u16, Vec<u32>> {
        let target_board = chess_to_board_array(target_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let total_games = self.index.header.db_game_count as usize;
        let move_set: HashSet<u16> = child_moves.iter().map(|m| m.0).collect();
        let mut samples_map: HashMap<u16, Vec<u32>> = HashMap::new();

        for gid in 0..total_games {
            if !move_set.is_empty()
                && move_set
                    .iter()
                    .all(|m| samples_map.get(m).map_or(0, |v| v.len()) >= max_samples_per_move)
            {
                break;
            }
            if let Some(moves) = self.index.get_game_moves(gid) {
                let mut replay = FastReplayState::new();
                let mut hit_ply: Option<usize> = None;
                if replay.board == target_board {
                    hit_ply = Some(0);
                } else {
                    for (ply_idx, &m) in moves.iter().enumerate() {
                        replay.apply_move(m);
                        if replay.white_pieces < target_w || replay.black_pieces < target_b {
                            break;
                        }
                        if replay.board == target_board {
                            hit_ply = Some(ply_idx + 1);
                            break;
                        }
                    }
                }
                if let Some(ply) = hit_ply {
                    if ply < moves.len() {
                        let next_move = moves[ply].0;
                        if move_set.contains(&next_move) {
                            let list = samples_map.entry(next_move).or_default();
                            if list.len() < max_samples_per_move {
                                list.push(gid as u32);
                            }
                        }
                    }
                }
            }
        }
        samples_map
    }

    /// Calculates a complete, dynamic opening tree report from the booster stream with rich W/D/L stats, ELO averages, and sample games
    /// Optionally calculates multi-ply continuation lines simultaneously in the exact same single parallel pass.
    pub fn calculate_opening_tree<F>(
        &self,
        target_fen: &str,
        target_game_ids: Option<&[usize]>,
        max_sample_ids: Option<usize>,
        meta_lookup: Option<F>,
        continuation_config: Option<&ContinuationQuery>,
    ) -> Result<Option<OpeningTreeReport>>
    where
        F: Fn(usize) -> Option<BoostGameMeta> + Sync + Send,
    {
        let fen_to_parse = if target_fen.trim().is_empty() {
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        } else {
            target_fen.trim()
        };

        let fen: Fen = fen_to_parse
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid FEN '{}': {}", fen_to_parse, e))?;
        let target_pos: Chess = fen
            .into_position(CastlingMode::Standard)
            .map_err(|e| anyhow::anyhow!("Invalid chess position from FEN: {}", e))?;

        let target_board = chess_to_board_array(&target_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let target_hash_val: Zobrist64 = target_pos.zobrist_hash(EnPassantMode::Legal);
        let target_hash = target_hash_val.0;

        let sample_cap = max_sample_ids.unwrap_or(20);

        #[derive(Debug, Clone, Default)]
        struct MoveStatsAcc {
            total_games: u32,
            white_wins: u32,
            draws: u32,
            black_wins: u32,
            white_elo_sum: u64,
            black_elo_sum: u64,
            elo_game_count: u32,
            max_year: Option<u16>,
            sample_game_ids: Vec<u32>,
        }

        #[derive(Debug, Clone, Default)]
        struct PathStats {
            games: u64,
            white_wins: u64,
            draws: u64,
            black_wins: u64,
        }

        #[derive(Debug, Clone, Default)]
        struct TreeAcc {
            total_games: u32,
            white_wins: u32,
            draws: u32,
            black_wins: u32,
            moves: HashMap<u16, MoveStatsAcc>,
            sample_game_ids: Vec<u32>,
            paths: HashMap<PackedPath256, PathStats>,
        }

        let total_games_in_index = self.index.game_count();

        let process_game = |gid: usize, acc: &mut TreeAcc| {
            let _entry = match self.index.get_game_entry(gid) {
                Some(e) if !e.is_deleted() && !e.is_custom_fen() => e,
                _ => return,
            };
            let moves = match self.index.get_game_moves(gid) {
                Some(m) => m,
                None => return,
            };

            let mut replay = FastReplayState::new();
            let mut hit_ply: Option<usize> = None;

            if replay.board == target_board {
                hit_ply = Some(0);
            } else {
                for (ply_idx, &m) in moves.iter().enumerate() {
                    replay.apply_move(m);
                    if replay.white_pieces < target_w || replay.black_pieces < target_b {
                        break;
                    }
                    if replay.board == target_board {
                        hit_ply = Some(ply_idx + 1);
                        break;
                    }
                }
            }

            if let Some(ply) = hit_ply {
                let meta = meta_lookup
                    .as_ref()
                    .and_then(|f| f(gid))
                    .unwrap_or_default();
                let (w_win, draw, b_win) = match meta.result {
                    1 => (1, 0, 0),
                    2 => (0, 0, 1),
                    3 => (0, 1, 0),
                    _ => (0, 0, 0),
                };

                acc.total_games += 1;
                acc.white_wins += w_win;
                acc.draws += draw;
                acc.black_wins += b_win;
                if acc.sample_game_ids.len() < sample_cap {
                    acc.sample_game_ids.push(gid as u32);
                }

                if ply < moves.len() {
                    let next_move = moves[ply].0;
                    let m_acc = acc.moves.entry(next_move).or_default();
                    m_acc.total_games += 1;
                    m_acc.white_wins += w_win;
                    m_acc.draws += draw;
                    m_acc.black_wins += b_win;
                    if meta.white_elo > 0 && meta.black_elo > 0 {
                        m_acc.white_elo_sum += meta.white_elo as u64;
                        m_acc.black_elo_sum += meta.black_elo as u64;
                        m_acc.elo_game_count += 1;
                    }
                    if let Some(y) = meta.year {
                        m_acc.max_year = Some(m_acc.max_year.map_or(y, |prev| prev.max(y)));
                    }
                    if m_acc.sample_game_ids.len() < sample_cap {
                        m_acc.sample_game_ids.push(gid as u32);
                    }

                    if let Some(cq) = continuation_config {
                        let end = (ply + cq.max_depth.min(MAX_PACKED_PATH_PLIES)).min(moves.len());
                        let path = PackedPath256::from_slice(&moves[ply..end]);
                        let st = acc.paths.entry(path).or_default();
                        st.games += 1;
                        st.white_wins += w_win as u64;
                        st.draws += draw as u64;
                        st.black_wins += b_win as u64;
                    }
                }
            }
        };

        let merged_acc: TreeAcc = if let Some(gids) = target_game_ids {
            gids.par_iter()
                .fold(TreeAcc::default, |mut acc, &gid| {
                    if gid < total_games_in_index {
                        process_game(gid, &mut acc);
                    }
                    acc
                })
                .reduce(TreeAcc::default, |mut a, b| {
                    a.total_games += b.total_games;
                    a.white_wins += b.white_wins;
                    a.draws += b.draws;
                    a.black_wins += b.black_wins;
                    if a.sample_game_ids.len() < sample_cap {
                        for id in b.sample_game_ids {
                            if a.sample_game_ids.len() >= sample_cap {
                                break;
                            }
                            a.sample_game_ids.push(id);
                        }
                    }
                    for (k, v) in b.moves {
                        let ma = a.moves.entry(k).or_default();
                        ma.total_games += v.total_games;
                        ma.white_wins += v.white_wins;
                        ma.draws += v.draws;
                        ma.black_wins += v.black_wins;
                        ma.white_elo_sum += v.white_elo_sum;
                        ma.black_elo_sum += v.black_elo_sum;
                        ma.elo_game_count += v.elo_game_count;
                        ma.max_year = match (ma.max_year, v.max_year) {
                            (Some(y1), Some(y2)) => Some(y1.max(y2)),
                            (Some(y1), None) => Some(y1),
                            (None, Some(y2)) => Some(y2),
                            (None, None) => None,
                        };
                        if ma.sample_game_ids.len() < sample_cap {
                            for id in v.sample_game_ids {
                                if ma.sample_game_ids.len() >= sample_cap {
                                    break;
                                }
                                ma.sample_game_ids.push(id);
                            }
                        }
                    }
                    for (k, v) in b.paths {
                        let st = a.paths.entry(k).or_default();
                        st.games += v.games;
                        st.white_wins += v.white_wins;
                        st.draws += v.draws;
                        st.black_wins += v.black_wins;
                    }
                    a
                })
        } else {
            (0..total_games_in_index)
                .into_par_iter()
                .fold(TreeAcc::default, |mut acc, gid| {
                    process_game(gid, &mut acc);
                    acc
                })
                .reduce(TreeAcc::default, |mut a, b| {
                    a.total_games += b.total_games;
                    a.white_wins += b.white_wins;
                    a.draws += b.draws;
                    a.black_wins += b.black_wins;
                    if a.sample_game_ids.len() < sample_cap {
                        for id in b.sample_game_ids {
                            if a.sample_game_ids.len() >= sample_cap {
                                break;
                            }
                            a.sample_game_ids.push(id);
                        }
                    }
                    for (k, v) in b.moves {
                        let ma = a.moves.entry(k).or_default();
                        ma.total_games += v.total_games;
                        ma.white_wins += v.white_wins;
                        ma.draws += v.draws;
                        ma.black_wins += v.black_wins;
                        ma.white_elo_sum += v.white_elo_sum;
                        ma.black_elo_sum += v.black_elo_sum;
                        ma.elo_game_count += v.elo_game_count;
                        ma.max_year = match (ma.max_year, v.max_year) {
                            (Some(y1), Some(y2)) => Some(y1.max(y2)),
                            (Some(y1), None) => Some(y1),
                            (None, Some(y2)) => Some(y2),
                            (None, None) => None,
                        };
                        if ma.sample_game_ids.len() < sample_cap {
                            for id in v.sample_game_ids {
                                if ma.sample_game_ids.len() >= sample_cap {
                                    break;
                                }
                                ma.sample_game_ids.push(id);
                            }
                        }
                    }
                    for (k, v) in b.paths {
                        let st = a.paths.entry(k).or_default();
                        st.games += v.games;
                        st.white_wins += v.white_wins;
                        st.draws += v.draws;
                        st.black_wins += v.black_wins;
                    }
                    a
                })
        };

        if merged_acc.total_games == 0 {
            return Ok(None);
        }

        let white_pct = (merged_acc.white_wins as f64 / merged_acc.total_games as f64) * 100.0;
        let draw_pct = (merged_acc.draws as f64 / merged_acc.total_games as f64) * 100.0;
        let black_pct = (merged_acc.black_wins as f64 / merged_acc.total_games as f64) * 100.0;

        let mut move_views: Vec<OpeningTreeMoveView> = merged_acc
            .moves
            .into_iter()
            .map(|(packed, m_stat)| {
                let bm = BoostMove(packed);
                let san = bm
                    .to_san_string(&target_pos)
                    .unwrap_or_else(|| bm.to_uci_string());
                let uci = bm.to_uci_string();
                let m_white_pct = (m_stat.white_wins as f64 / m_stat.total_games as f64) * 100.0;
                let m_draw_pct = (m_stat.draws as f64 / m_stat.total_games as f64) * 100.0;
                let m_black_pct = (m_stat.black_wins as f64 / m_stat.total_games as f64) * 100.0;

                let avg_white_elo = if m_stat.elo_game_count > 0 {
                    Some((m_stat.white_elo_sum / m_stat.elo_game_count as u64) as u32)
                } else {
                    None
                };
                let avg_black_elo = if m_stat.elo_game_count > 0 {
                    Some((m_stat.black_elo_sum / m_stat.elo_game_count as u64) as u32)
                } else {
                    None
                };

                OpeningTreeMoveView {
                    san,
                    uci,
                    total_games: m_stat.total_games,
                    white_pct: m_white_pct,
                    draw_pct: m_draw_pct,
                    black_pct: m_black_pct,
                    white_wins: m_stat.white_wins,
                    draws: m_stat.draws,
                    black_wins: m_stat.black_wins,
                    avg_white_elo,
                    avg_black_elo,
                    last_played: m_stat.max_year.map(|y| y.to_string()),
                    sample_game_ids: m_stat.sample_game_ids,
                }
            })
            .collect();

        move_views.sort_by_key(|m| std::cmp::Reverse(m.total_games));

        let continuations_res = if let Some(cq) = continuation_config {
            if merged_acc.total_games > 0 {
                let start_fullmove = parse_fen_fullmove(fen_to_parse);
                let mut lines = Vec::new();
                for (path, stats) in merged_acc.paths {
                    let percentage = (stats.games as f64 / merged_acc.total_games as f64) * 100.0;
                    if stats.games >= cq.min_games && percentage >= cq.min_percentage {
                        let mut sim_pos = target_pos.clone();
                        let mut san_moves = Vec::with_capacity(path.len as usize);

                        for bm in path.iter() {
                            if let Some(m) = bm.to_shakmaty_move(&sim_pos) {
                                let san_plus = shakmaty::san::SanPlus::from_move_and_play_unchecked(
                                    &mut sim_pos,
                                    &m,
                                );
                                san_moves.push(san_plus.to_string());
                            } else {
                                san_moves.push(bm.to_uci_string());
                            }
                        }

                        let formatted =
                            format_continuation_moves(&target_pos, start_fullmove, &san_moves);
                        lines.push(ContinuationLine {
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

                lines.sort_by(|a, b| {
                    b.games
                        .cmp(&a.games)
                        .then_with(|| b.moves.len().cmp(&a.moves.len()))
                        .then_with(|| a.moves.cmp(&b.moves))
                });

                if lines.len() > cq.max_lines {
                    lines.truncate(cq.max_lines);
                }

                Some(lines)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Some(OpeningTreeReport {
            fen: fen_to_parse.to_string(),
            zobrist_hash: target_hash,
            total_games: merged_acc.total_games,
            white_wins: merged_acc.white_wins,
            draws: merged_acc.draws,
            black_wins: merged_acc.black_wins,
            white_pct,
            draw_pct,
            black_pct,
            moves: move_views,
            sample_game_ids: merged_acc.sample_game_ids,
            sample_games: Vec::new(),
            continuations: continuations_res,
        }))
    }

    /// Calculates rich continuation lines and branch tree directly from the booster stream
    pub fn calculate_continuations<F>(
        &self,
        query: &ContinuationQuery,
        target_game_ids: Option<&[usize]>,
        meta_lookup: Option<F>,
    ) -> Result<Option<ContinuationResult>>
    where
        F: Fn(usize) -> Option<BoostGameMeta> + Sync + Send,
    {
        let start_pos = query.validate()?;
        let target_board = chess_to_board_array(&start_pos);
        let (target_w, target_b) = FastReplayState::count_pieces(&target_board);
        let start_fullmove = parse_fen_fullmove(&query.position);

        #[derive(Debug, Clone, Default)]
        struct PathStats {
            games: u64,
            white_wins: u64,
            draws: u64,
            black_wins: u64,
        }

        #[derive(Debug, Clone, Default)]
        struct ContAcc {
            total_processed: u64,
            games_reaching: u64,
            paths: HashMap<PackedPath256, PathStats>,
        }

        let total_games_in_index = self.index.game_count();
        let max_depth = query.max_depth;

        let process_game = |gid: usize, acc: &mut ContAcc| {
            acc.total_processed += 1;
            let _entry = match self.index.get_game_entry(gid) {
                Some(e) if !e.is_deleted() && !e.is_custom_fen() => e,
                _ => return,
            };
            let moves = match self.index.get_game_moves(gid) {
                Some(m) => m,
                None => return,
            };

            let mut replay = FastReplayState::new();
            let mut hit_ply: Option<usize> = None;

            if replay.board == target_board {
                hit_ply = Some(0);
            } else {
                for (ply_idx, &m) in moves.iter().enumerate() {
                    replay.apply_move(m);
                    if replay.white_pieces < target_w || replay.black_pieces < target_b {
                        break;
                    }
                    if replay.board == target_board {
                        hit_ply = Some(ply_idx + 1);
                        break;
                    }
                }
            }

            if let Some(ply) = hit_ply {
                acc.games_reaching += 1;
                let meta = meta_lookup
                    .as_ref()
                    .and_then(|f| f(gid))
                    .unwrap_or_default();
                let (w_win, draw, b_win) = match meta.result {
                    1 => (1u64, 0u64, 0u64),
                    2 => (0u64, 0u64, 1u64),
                    3 => (0u64, 1u64, 0u64),
                    _ => (0u64, 0u64, 0u64),
                };

                if ply < moves.len() {
                    let end = (ply + max_depth.min(MAX_PACKED_PATH_PLIES)).min(moves.len());
                    let path = PackedPath256::from_slice(&moves[ply..end]);
                    let st = acc.paths.entry(path).or_default();
                    st.games += 1;
                    st.white_wins += w_win;
                    st.draws += draw;
                    st.black_wins += b_win;
                }
            }
        };

        let merged_acc: ContAcc = if let Some(gids) = target_game_ids {
            gids.par_iter()
                .fold(ContAcc::default, |mut acc, &gid| {
                    if gid < total_games_in_index {
                        process_game(gid, &mut acc);
                    }
                    acc
                })
                .reduce(ContAcc::default, |mut a, b| {
                    a.total_processed += b.total_processed;
                    a.games_reaching += b.games_reaching;
                    for (k, v) in b.paths {
                        let st = a.paths.entry(k).or_default();
                        st.games += v.games;
                        st.white_wins += v.white_wins;
                        st.draws += v.draws;
                        st.black_wins += v.black_wins;
                    }
                    a
                })
        } else {
            (0..total_games_in_index)
                .into_par_iter()
                .fold(ContAcc::default, |mut acc, gid| {
                    process_game(gid, &mut acc);
                    acc
                })
                .reduce(ContAcc::default, |mut a, b| {
                    a.total_processed += b.total_processed;
                    a.games_reaching += b.games_reaching;
                    for (k, v) in b.paths {
                        let st = a.paths.entry(k).or_default();
                        st.games += v.games;
                        st.white_wins += v.white_wins;
                        st.draws += v.draws;
                        st.black_wins += v.black_wins;
                    }
                    a
                })
        };

        let mut lines = Vec::new();
        if merged_acc.games_reaching > 0 {
            for (path, stats) in merged_acc.paths {
                let percentage = (stats.games as f64 / merged_acc.games_reaching as f64) * 100.0;
                if stats.games >= query.min_games && percentage >= query.min_percentage {
                    let mut sim_pos = start_pos.clone();
                    let mut san_moves = Vec::with_capacity(path.len as usize);

                    for bm in path.iter() {
                        if let Some(m) = bm.to_shakmaty_move(&sim_pos) {
                            let san_plus = shakmaty::san::SanPlus::from_move_and_play_unchecked(
                                &mut sim_pos,
                                &m,
                            );
                            san_moves.push(san_plus.to_string());
                        } else {
                            san_moves.push(bm.to_uci_string());
                        }
                    }

                    let formatted =
                        format_continuation_moves(&start_pos, start_fullmove, &san_moves);
                    lines.push(ContinuationLine {
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

            lines.sort_by(|a, b| {
                b.games
                    .cmp(&a.games)
                    .then_with(|| b.moves.len().cmp(&a.moves.len()))
                    .then_with(|| a.moves.cmp(&b.moves))
            });

            if lines.len() > query.max_lines {
                lines.truncate(query.max_lines);
            }
        }

        Ok(Some(ContinuationResult {
            starting_fen: Fen::from_position(start_pos, EnPassantMode::Legal).to_string(),
            total_games_processed: merged_acc.total_processed,
            games_reaching_position: merged_acc.games_reaching,
            lines,
            tree: None,
        }))
    }
}
