use anyhow::Result;
use rayon::prelude::*;
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess, Position, Role, Square};

use super::codec::MmapBoostIndex;
use super::types::BoostMove;

/// Lightweight 64-byte scratchpad board for sub-nanosecond move application
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FastReplayState {
    /// 0 = Empty, 1..6 = White (P, N, B, R, Q, K), 9..14 = Black (P, N, B, R, Q, K)
    pub board: [u8; 64],
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

        Self { board }
    }

    #[inline(always)]
    pub fn apply_move(&mut self, m: BoostMove) {
        let from = m.from();
        let to = m.to();
        let flags = m.flags();
        let piece = self.board[from];

        self.board[from] = 0;

        if flags <= 0x1 || flags == 0x4 {
            // Quiet move, double pawn push, or normal capture
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
                }
            } else {
                // Black captured white pawn on rank 4
                if to + 8 < 64 {
                    self.board[to + 8] = 0;
                }
            }
        } else if flags >= 0x8 {
            // Promotion
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

    /// Scans games in parallel for board positions reaching a target FEN
    pub fn search_position(
        &self,
        target_fen: &str,
        max_ply: Option<usize>,
    ) -> Result<Vec<BoostMatch>> {
        let fen: Fen = target_fen
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid FEN: {}", e))?;
        let target_pos: Chess = fen
            .into_position(CastlingMode::Chess960)
            .map_err(|e| anyhow::anyhow!("Invalid position: {}", e))?;

        let target_board = chess_to_board_array(&target_pos);
        let total_games = self.index.game_count();

        let matches: Vec<BoostMatch> = (0..total_games)
            .into_par_iter()
            .filter_map(|gid| {
                let entry = self.index.get_game_entry(gid)?;
                if entry.is_deleted() {
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
                    matching_plies.push(0);
                }

                for (ply_idx, &m) in moves[..limit].iter().enumerate() {
                    replay.apply_move(m);
                    if replay.board == target_board {
                        matching_plies.push(ply_idx + 1);
                    }
                }

                if !matching_plies.is_empty() {
                    Some(BoostMatch {
                        game_id: gid,
                        matching_plies,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(matches)
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
        let total_games = self.index.game_count();

        let move_counts = (0..total_games)
            .into_par_iter()
            .fold(
                std::collections::HashMap::<u16, u32>::new,
                |mut acc, gid| {
                    let _entry = match self.index.get_game_entry(gid) {
                        Some(e) if !e.is_deleted() => e,
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
                        if replay.board == target_board && ply_idx + 1 < moves.len() {
                            *acc.entry(moves[ply_idx + 1].0).or_insert(0) += 1;
                        }
                    }

                    acc
                },
            )
            .reduce(std::collections::HashMap::new, |mut map1, map2| {
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
        let total_games = self.index.game_count();

        let line_counts = (0..total_games)
            .into_par_iter()
            .fold(
                std::collections::HashMap::<Vec<u16>, u32>::new,
                |mut acc, gid| {
                    let _entry = match self.index.get_game_entry(gid) {
                        Some(e) if !e.is_deleted() => e,
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
                        let seq: Vec<u16> = moves[0..end].iter().map(|m| m.0).collect();
                        *acc.entry(seq).or_insert(0) += 1;
                    }

                    for (ply_idx, &m) in moves[..limit].iter().enumerate() {
                        replay.apply_move(m);
                        if replay.board == target_board && ply_idx + 1 < moves.len() {
                            let start = ply_idx + 1;
                            let end = (start + depth).min(moves.len());
                            let seq: Vec<u16> = moves[start..end].iter().map(|m| m.0).collect();
                            *acc.entry(seq).or_insert(0) += 1;
                        }
                    }

                    acc
                },
            )
            .reduce(std::collections::HashMap::new, |mut map1, map2| {
                for (k, v) in map2 {
                    *map1.entry(k).or_insert(0) += v;
                }
                map1
            });

        let mut sorted: Vec<(Vec<BoostMove>, u32)> = line_counts
            .into_iter()
            .map(|(seq, count)| (seq.into_iter().map(BoostMove).collect(), count))
            .collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.1));

        Ok(sorted)
    }
}
