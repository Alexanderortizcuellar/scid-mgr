use shakmaty::{Bitboard, Chess, Position, Square};
use std::collections::HashMap;

use super::parser::helpers::expand_diagonal_ray;
use super::query::{ComparisonOp, SetPredicate, SquareContent, SquareSetExpr};

/// Evaluator for First-Class Square Set Expressions and Set Predicates
pub struct SquareSetEvaluator;

impl SquareSetEvaluator {
    /// Evaluate a `SquareSetExpr` to a 64-bit Bitboard on a given position and variable environment
    pub fn eval_expr(expr: &SquareSetExpr, pos: &Chess, env: &HashMap<String, Square>) -> Bitboard {
        let board = pos.board();
        match expr {
            SquareSetExpr::Piece(content) => match content {
                SquareContent::Empty => !board.occupied(),
                SquareContent::Occupied => board.occupied(),
                SquareContent::Piece(piece) => board.by_piece(*piece),
                SquareContent::Role(role) => board.by_role(*role),
                SquareContent::Color(color) => board.by_color(*color),
                SquareContent::AnyOf(pieces) => {
                    let mut bb = Bitboard::EMPTY;
                    for p in pieces {
                        bb |= board.by_piece(*p);
                    }
                    bb
                }
                SquareContent::NoneOf(pieces) => {
                    let mut bb = board.occupied();
                    for p in pieces {
                        bb &= !board.by_piece(*p);
                    }
                    bb
                }
            },
            SquareSetExpr::Squares(bb) => *bb,
            SquareSetExpr::Variable(name) => {
                if let Some(&sq) = env.get(name) {
                    Bitboard::from_square(sq)
                } else {
                    Bitboard::EMPTY
                }
            }
            SquareSetExpr::Intersection(left, right) => {
                let bb_left = Self::eval_expr(left, pos, env);
                let bb_right = Self::eval_expr(right, pos, env);
                bb_left & bb_right
            }
            SquareSetExpr::Union(left, right) => {
                let bb_left = Self::eval_expr(left, pos, env);
                let bb_right = Self::eval_expr(right, pos, env);
                bb_left | bb_right
            }
            SquareSetExpr::Difference(left, right) => {
                let bb_left = Self::eval_expr(left, pos, env);
                let bb_right = Self::eval_expr(right, pos, env);
                bb_left & !bb_right
            }
            SquareSetExpr::Complement(inner) => {
                let bb_inner = Self::eval_expr(inner, pos, env);
                !bb_inner
            }
            SquareSetExpr::Attacks { attacker, target } => {
                let atk_bb = Self::eval_expr(attacker, pos, env);
                let tgt_bb = Self::eval_expr(target, pos, env);
                let mut attacked_targets = Bitboard::EMPTY;

                for from_sq in atk_bb {
                    let attacks = board.attacks_from(from_sq);
                    attacked_targets |= attacks & tgt_bb;
                }
                attacked_targets
            }
            SquareSetExpr::Attackers { attacker, target } => {
                let atk_bb = Self::eval_expr(attacker, pos, env);
                let tgt_bb = Self::eval_expr(target, pos, env);
                let mut attacking_pieces = Bitboard::EMPTY;

                for from_sq in atk_bb {
                    let attacks = board.attacks_from(from_sq);
                    if !(attacks & tgt_bb).is_empty() {
                        attacking_pieces.add(from_sq);
                    }
                }
                attacking_pieces
            }
            SquareSetExpr::Ray { direction, origin } => {
                let orig_bb = Self::eval_expr(origin, pos, env);
                let mut ray_bb = Bitboard::EMPTY;

                for from_sq in orig_bb {
                    let expanded = direction.expand_square(from_sq, 1, 7);
                    for sq in expanded {
                        ray_bb.add(sq);
                    }
                }
                ray_bb
            }
            SquareSetExpr::Between { from, to } => {
                let from_bb = Self::eval_expr(from, pos, env);
                let to_bb = Self::eval_expr(to, pos, env);
                let mut between_bb = Bitboard::EMPTY;

                for sq1 in from_bb {
                    for sq2 in to_bb {
                        if sq1 == sq2 {
                            continue;
                        }
                        if let Some(ray) = expand_diagonal_ray(sq1, sq2) {
                            for sq in ray {
                                if sq != sq1 && sq != sq2 {
                                    between_bb.add(sq);
                                }
                            }
                        }
                    }
                }
                between_bb
            }
            SquareSetExpr::Shift {
                direction,
                min_dist,
                max_dist,
                expr,
            } => {
                let orig_bb = Self::eval_expr(expr, pos, env);
                let mut shifted_bb = Bitboard::EMPTY;
                for from_sq in orig_bb {
                    let expanded = direction.expand_square(from_sq, *min_dist, *max_dist);
                    for sq in expanded {
                        shifted_bb.add(sq);
                    }
                }
                shifted_bb
            }
        }
    }

    /// Evaluate a `SetPredicate` against a Shakmaty position and variable environment
    pub fn matches_with_env(
        pred: &SetPredicate,
        pos: &Chess,
        env: &HashMap<String, Square>,
    ) -> bool {
        match pred {
            SetPredicate::NonEmpty(expr) => {
                let bb = Self::eval_expr(expr, pos, env);
                !bb.is_empty()
            }
            SetPredicate::CountComparison { expr, op, count } => {
                let bb = Self::eval_expr(expr, pos, env);
                let actual = bb.count();
                match op {
                    ComparisonOp::Equal => actual == *count,
                    ComparisonOp::NotEqual => actual != *count,
                    ComparisonOp::GreaterThan => actual > *count,
                    ComparisonOp::GreaterThanOrEqual => actual >= *count,
                    ComparisonOp::LessThan => actual < *count,
                    ComparisonOp::LessThanOrEqual => actual <= *count,
                    _ => actual == *count,
                }
            }
            SetPredicate::SetComparison { left, op, right } => {
                let left_count = Self::eval_expr(left, pos, env).count();
                let right_count = Self::eval_expr(right, pos, env).count();
                match op {
                    ComparisonOp::Equal => left_count == right_count,
                    ComparisonOp::NotEqual => left_count != right_count,
                    ComparisonOp::GreaterThan => left_count > right_count,
                    ComparisonOp::GreaterThanOrEqual => left_count >= right_count,
                    ComparisonOp::LessThan => left_count < right_count,
                    ComparisonOp::LessThanOrEqual => left_count <= right_count,
                    _ => left_count == right_count,
                }
            }
        }
    }

    /// Evaluate a `SetPredicate` against a Shakmaty position with empty environment
    pub fn matches(pred: &SetPredicate, pos: &Chess) -> bool {
        Self::matches_with_env(pred, pos, &HashMap::new())
    }
}
