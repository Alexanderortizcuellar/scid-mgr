use super::query::{ComparisonOp, PawnPredicate};
use shakmaty::{Bitboard, Color, File, Piece, Rank, Role, Square};

const PASSED_PAWN_MASKS_WHITE: [Bitboard; 64] = {
    let mut table = [Bitboard::EMPTY; 64];
    let mut sq = 0;
    while sq < 64 {
        let f = sq % 8;
        let r = sq / 8;
        let mut bb = 0u64;
        let mut r_curr = r + 1;
        while r_curr < 8 {
            if f > 0 {
                bb |= 1u64 << (r_curr * 8 + (f - 1));
            }
            bb |= 1u64 << (r_curr * 8 + f);
            if f < 7 {
                bb |= 1u64 << (r_curr * 8 + (f + 1));
            }
            r_curr += 1;
        }
        table[sq] = Bitboard(bb);
        sq += 1;
    }
    table
};

const PASSED_PAWN_MASKS_BLACK: [Bitboard; 64] = {
    let mut table = [Bitboard::EMPTY; 64];
    let mut sq = 0;
    while sq < 64 {
        let f = sq % 8;
        let r = sq / 8;
        let mut bb = 0u64;
        let mut r_curr = 0;
        while r_curr < r {
            if f > 0 {
                bb |= 1u64 << (r_curr * 8 + (f - 1));
            }
            bb |= 1u64 << (r_curr * 8 + f);
            if f < 7 {
                bb |= 1u64 << (r_curr * 8 + (f + 1));
            }
            r_curr += 1;
        }
        table[sq] = Bitboard(bb);
        sq += 1;
    }
    table
};

const ADJACENT_FILES_MASKS: [Bitboard; 8] = {
    let mut table = [Bitboard::EMPTY; 8];
    let mut f = 0;
    while f < 8 {
        let mut bb = 0u64;
        let mut r = 0;
        while r < 8 {
            if f > 0 {
                bb |= 1u64 << (r * 8 + (f - 1));
            }
            if f < 7 {
                bb |= 1u64 << (r * 8 + (f + 1));
            }
            r += 1;
        }
        table[f] = Bitboard(bb);
        f += 1;
    }
    table
};

/// Bitboard-accelerated pawn structure evaluator
pub struct PawnEvaluator;

impl PawnEvaluator {
    /// Evaluates a PawnPredicate against a board state
    pub fn matches(pred: &PawnPredicate, board: &shakmaty::Board) -> bool {
        match pred {
            PawnPredicate::PassedPawns { color, op, count } => {
                let actual = Self::count_passed_pawns(board, *color);
                compare_usize(actual, *count, *op)
            }
            PawnPredicate::IsolatedPawns { color, op, count } => {
                let actual = Self::count_isolated_pawns(board, *color);
                compare_usize(actual, *count, *op)
            }
            PawnPredicate::DoubledPawns { color, op, count } => {
                let actual = Self::count_doubled_pawns(board, *color);
                compare_usize(actual, *count, *op)
            }
            PawnPredicate::BackwardPawns { color, op, count } => {
                let actual = Self::count_backward_pawns(board, *color);
                compare_usize(actual, *count, *op)
            }
            PawnPredicate::PawnIslands { color, op, count } => {
                let actual = Self::count_pawn_islands(board, *color);
                compare_usize(actual, *count, *op)
            }
        }
    }

    /// Count passed pawns for a given color using precomputed bitboard lookup tables
    pub fn count_passed_pawns(board: &shakmaty::Board, color: Color) -> usize {
        let friendly_pawns = board.by_piece(Piece {
            color,
            role: Role::Pawn,
        });
        let enemy_pawns = board.by_piece(Piece {
            color: color.other(),
            role: Role::Pawn,
        });
        let mut count = 0;

        match color {
            Color::White => {
                for sq in friendly_pawns {
                    if (enemy_pawns & PASSED_PAWN_MASKS_WHITE[sq as usize]).is_empty() {
                        count += 1;
                    }
                }
            }
            Color::Black => {
                for sq in friendly_pawns {
                    if (enemy_pawns & PASSED_PAWN_MASKS_BLACK[sq as usize]).is_empty() {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Count isolated pawns (pawns with no friendly pawns on adjacent files)
    pub fn count_isolated_pawns(board: &shakmaty::Board, color: Color) -> usize {
        let friendly_pawns = board.by_piece(Piece {
            color,
            role: Role::Pawn,
        });
        let mut count = 0;

        for sq in friendly_pawns {
            if (friendly_pawns & ADJACENT_FILES_MASKS[sq.file() as usize]).is_empty() {
                count += 1;
            }
        }
        count
    }

    #[inline]
    fn pawn_file_mask(pawns: Bitboard) -> u8 {
        let mut mask = 0u8;
        let mut f = 0;
        while f < 8 {
            let file_bb = Bitboard(0x0101010101010101u64 << f);
            if !(pawns & file_bb).is_empty() {
                mask |= 1 << f;
            }
            f += 1;
        }
        mask
    }

    /// Count doubled pawns (count of pawns on a file beyond the first pawn)
    pub fn count_doubled_pawns(board: &shakmaty::Board, color: Color) -> usize {
        let friendly_pawns = board.by_piece(Piece {
            color,
            role: Role::Pawn,
        });
        let occupied_files = Self::pawn_file_mask(friendly_pawns).count_ones() as usize;
        friendly_pawns.count().saturating_sub(occupied_files)
    }

    /// Count backward pawns
    pub fn count_backward_pawns(board: &shakmaty::Board, color: Color) -> usize {
        let friendly_pawns = board.by_piece(Piece {
            color,
            role: Role::Pawn,
        });
        let enemy_pawns = board.by_piece(Piece {
            color: color.other(),
            role: Role::Pawn,
        });
        let mut count = 0;

        for sq in friendly_pawns {
            let file = sq.file();
            let rank = sq.rank();
            let adj_mask = ADJACENT_FILES_MASKS[file as usize];

            let is_behind = match color {
                Color::White => {
                    let friendly_behind =
                        (friendly_pawns & adj_mask & rank_span_below_or_equal(rank)).is_empty();
                    let advance_sq = sq.offset(8);
                    let advance_controlled = advance_sq
                        .map(|adv| {
                            let adv_attacks = pawn_attacks(adv, Color::Black);
                            !(enemy_pawns & adv_attacks).is_empty()
                        })
                        .unwrap_or(false);
                    friendly_behind && advance_controlled
                }
                Color::Black => {
                    let friendly_behind =
                        (friendly_pawns & adj_mask & rank_span_above_or_equal(rank)).is_empty();
                    let advance_sq = sq.offset(-8);
                    let advance_controlled = advance_sq
                        .map(|adv| {
                            let adv_attacks = pawn_attacks(adv, Color::White);
                            !(enemy_pawns & adv_attacks).is_empty()
                        })
                        .unwrap_or(false);
                    friendly_behind && advance_controlled
                }
            };

            if is_behind {
                count += 1;
            }
        }
        count
    }

    /// Count pawn islands (number of contiguous groups of files containing friendly pawns)
    pub fn count_pawn_islands(board: &shakmaty::Board, color: Color) -> usize {
        let friendly_pawns = board.by_piece(Piece {
            color,
            role: Role::Pawn,
        });
        let mask = Self::pawn_file_mask(friendly_pawns);
        (mask & !(mask >> 1)).count_ones() as usize
    }
}

fn rank_span_above_or_equal(rank: Rank) -> Bitboard {
    let mut bb = Bitboard::EMPTY;
    for r in (rank as usize)..8 {
        bb |= Bitboard::from(Rank::ALL[r]);
    }
    bb
}

fn rank_span_below_or_equal(rank: Rank) -> Bitboard {
    let mut bb = Bitboard::EMPTY;
    for r in 0..=(rank as usize) {
        bb |= Bitboard::from(Rank::ALL[r]);
    }
    bb
}

fn pawn_attacks(sq: Square, attacking_color: Color) -> Bitboard {
    let mut bb = Bitboard::EMPTY;
    match attacking_color {
        Color::White => {
            // White attacks upward
            if sq.file() != File::A {
                if let Some(l) = sq.offset(7) {
                    bb |= Bitboard::from_square(l);
                }
            }
            if sq.file() != File::H {
                if let Some(r) = sq.offset(9) {
                    bb |= Bitboard::from_square(r);
                }
            }
        }
        Color::Black => {
            // Black attacks downward
            if sq.file() != File::A {
                if let Some(l) = sq.offset(-9) {
                    bb |= Bitboard::from_square(l);
                }
            }
            if sq.file() != File::H {
                if let Some(r) = sq.offset(-7) {
                    bb |= Bitboard::from_square(r);
                }
            }
        }
    }
    bb
}

fn compare_usize(actual: usize, expected: usize, op: ComparisonOp) -> bool {
    match op {
        ComparisonOp::Equal => actual == expected,
        ComparisonOp::NotEqual => actual != expected,
        ComparisonOp::GreaterThan => actual > expected,
        ComparisonOp::GreaterThanOrEqual => actual >= expected,
        ComparisonOp::LessThan => actual < expected,
        ComparisonOp::LessThanOrEqual => actual <= expected,
        _ => false,
    }
}
