use shakmaty::fen::Fen;
use shakmaty::zobrist::ZobristHash;
use shakmaty::{
    Bitboard, CastlingMode, CastlingSide, Chess, Color, EnPassantMode, Piece, Position, Role,
    Square,
};
use std::str::FromStr;

use super::query::{ComparisonOp, MaterialPredicate, PositionPattern, SquareContent};

/// Position pattern matching engine for evaluating board positions
pub struct PositionMatcher;

impl PositionMatcher {
    /// Evaluate a PositionPattern against a Shakmaty board state at ply 0
    pub fn matches(pattern: &PositionPattern, pos: &Chess) -> bool {
        Self::matches_at_ply(pattern, pos, 0)
    }

    /// Evaluate a PositionPattern against a Shakmaty board state at a specific timeline ply
    pub fn matches_at_ply(pattern: &PositionPattern, pos: &Chess, ply: usize) -> bool {
        match pattern {
            PositionPattern::Ply { op, value } => compare_usize(ply, *value, *op),
            PositionPattern::MoveNumber { op, value } => {
                let movenum = if ply == 0 { 1 } else { ply.div_ceil(2) };
                compare_usize(movenum, *value, *op)
            }
            PositionPattern::ExactFen(fen_str) => {
                if fen_str.contains('*') || fen_str.contains('?') || fen_str.split('/').count() < 8
                {
                    return match_wildcard_fen(pos, fen_str);
                }
                if let Ok(fen) = Fen::from_str(fen_str.trim()) {
                    if let Ok(target_pos) = fen.into_position::<Chess>(CastlingMode::Standard) {
                        let c1 = pos.castles();
                        let c2 = target_pos.castles();
                        let same_castles = c1.has(Color::White, CastlingSide::KingSide)
                            == c2.has(Color::White, CastlingSide::KingSide)
                            && c1.has(Color::White, CastlingSide::QueenSide)
                                == c2.has(Color::White, CastlingSide::QueenSide)
                            && c1.has(Color::Black, CastlingSide::KingSide)
                                == c2.has(Color::Black, CastlingSide::KingSide)
                            && c1.has(Color::Black, CastlingSide::QueenSide)
                                == c2.has(Color::Black, CastlingSide::QueenSide);

                        return pos.board() == target_pos.board()
                            && pos.turn() == target_pos.turn()
                            && same_castles;
                    }
                }
                match_wildcard_fen(pos, fen_str)
            }
            PositionPattern::PiecePlacement(placement_str) => {
                match_wildcard_fen(pos, placement_str)
            }
            PositionPattern::ZobristHash(target_hash) => {
                let current_hash = pos
                    .zobrist_hash::<shakmaty::zobrist::Zobrist64>(EnPassantMode::Legal)
                    .0;
                current_hash == *target_hash
            }
            PositionPattern::Squares(square_map) => {
                for (&sq, content) in square_map {
                    if !match_square_content(pos, sq, content) {
                        return false;
                    }
                }
                true
            }
            PositionPattern::PieceCount {
                content,
                squares,
                op,
                count,
            } => {
                let actual_count = count_square_content(pos.board(), squares.as_deref(), content);
                match op {
                    ComparisonOp::Equal => actual_count == *count,
                    ComparisonOp::NotEqual => actual_count != *count,
                    ComparisonOp::GreaterThan => actual_count > *count,
                    ComparisonOp::GreaterThanOrEqual => actual_count >= *count,
                    ComparisonOp::LessThan => actual_count < *count,
                    ComparisonOp::LessThanOrEqual => actual_count <= *count,
                    _ => actual_count == *count,
                }
            }
            PositionPattern::Turn(expected_turn) => pos.turn() == *expected_turn,
            PositionPattern::Castling {
                color,
                kingside,
                queenside,
            } => {
                let c = pos.castles();
                if let Some(ks) = kingside {
                    if c.has(*color, CastlingSide::KingSide) != *ks {
                        return false;
                    }
                }
                if let Some(qs) = queenside {
                    if c.has(*color, CastlingSide::QueenSide) != *qs {
                        return false;
                    }
                }
                true
            }
            PositionPattern::BoardState {
                is_check,
                is_checkmate,
                is_stalemate,
            } => {
                if let Some(chk) = is_check {
                    if pos.is_check() != *chk {
                        return false;
                    }
                }
                if let Some(mate) = is_checkmate {
                    if *mate && !pos.is_check() {
                        return false;
                    }
                    if pos.is_checkmate() != *mate {
                        return false;
                    }
                }
                if let Some(stale) = is_stalemate {
                    if *stale && pos.is_check() {
                        return false;
                    }
                    if pos.is_stalemate() != *stale {
                        return false;
                    }
                }
                true
            }
            PositionPattern::Attack { from, to } => {
                let attacks = pos.board().attacks_from(*from);
                attacks.contains(*to)
            }
            PositionPattern::IsAttacked { square, by_color } => {
                let attackers = pos
                    .board()
                    .attacks_to(*square, *by_color, pos.board().occupied());
                !attackers.is_empty()
            }
            PositionPattern::MultiSquare { content, squares } => squares
                .iter()
                .any(|&sq| match_square_content(pos, sq, content)),
            PositionPattern::VariableSquareFilter {
                var_name: _,
                squares: _,
            } => {
                // If evaluated without environment, return true (since existence is handled in VariableBinding)
                true
            }
            PositionPattern::Symmetric { pattern, symmetry } => {
                super::transform::TransformMatcher::matches_with_symmetry(pattern, pos, *symmetry)
            }
        }
    }

    /// Evaluate a PositionPattern against a Shakmaty board state with variable environment
    pub fn matches_at_ply_with_env(
        pattern: &PositionPattern,
        pos: &Chess,
        ply: usize,
        env: &std::collections::HashMap<String, Square>,
    ) -> bool {
        match pattern {
            PositionPattern::VariableSquareFilter { var_name, squares } => {
                if let Some(&sq) = env.get(var_name) {
                    squares.contains(&sq)
                } else {
                    false
                }
            }
            _ => Self::matches_at_ply(pattern, pos, ply),
        }
    }

    /// Evaluate hardware bitboard material composition
    pub fn matches_material(pred: &MaterialPredicate, pos: &Chess) -> bool {
        let board = pos.board();

        if let Some(wp) = pred.white_pawns {
            if board
                .by_piece(Piece {
                    color: Color::White,
                    role: Role::Pawn,
                })
                .count()
                != wp
            {
                return false;
            }
        }
        if let Some(wn) = pred.white_knights {
            if board
                .by_piece(Piece {
                    color: Color::White,
                    role: Role::Knight,
                })
                .count()
                != wn
            {
                return false;
            }
        }
        if let Some(wb) = pred.white_bishops {
            if board
                .by_piece(Piece {
                    color: Color::White,
                    role: Role::Bishop,
                })
                .count()
                != wb
            {
                return false;
            }
        }
        if let Some(wr) = pred.white_rooks {
            if board
                .by_piece(Piece {
                    color: Color::White,
                    role: Role::Rook,
                })
                .count()
                != wr
            {
                return false;
            }
        }
        if let Some(wq) = pred.white_queens {
            if board
                .by_piece(Piece {
                    color: Color::White,
                    role: Role::Queen,
                })
                .count()
                != wq
            {
                return false;
            }
        }

        if let Some(bp) = pred.black_pawns {
            if board
                .by_piece(Piece {
                    color: Color::Black,
                    role: Role::Pawn,
                })
                .count()
                != bp
            {
                return false;
            }
        }
        if let Some(bn) = pred.black_knights {
            if board
                .by_piece(Piece {
                    color: Color::Black,
                    role: Role::Knight,
                })
                .count()
                != bn
            {
                return false;
            }
        }
        if let Some(bb) = pred.black_bishops {
            if board
                .by_piece(Piece {
                    color: Color::Black,
                    role: Role::Bishop,
                })
                .count()
                != bb
            {
                return false;
            }
        }
        if let Some(br) = pred.black_rooks {
            if board
                .by_piece(Piece {
                    color: Color::Black,
                    role: Role::Rook,
                })
                .count()
                != br
            {
                return false;
            }
        }
        if let Some(bq) = pred.black_queens {
            if board
                .by_piece(Piece {
                    color: Color::Black,
                    role: Role::Queen,
                })
                .count()
                != bq
            {
                return false;
            }
        }

        if let Some((op, diff)) = pred.material_difference {
            let w_pts = calculate_power(board, Color::White);
            let b_pts = calculate_power(board, Color::Black);
            let actual_diff = w_pts - b_pts;
            if !compare_i32(actual_diff, diff, op) {
                return false;
            }
        }

        let white_bishops = board.by_piece(Piece {
            color: Color::White,
            role: Role::Bishop,
        });
        let black_bishops = board.by_piece(Piece {
            color: Color::Black,
            role: Role::Bishop,
        });
        let all_bishops = board.by_role(Role::Bishop);

        if let Some(wlb) = pred.white_light_bishops {
            if (white_bishops & Bitboard::LIGHT_SQUARES).count() != wlb {
                return false;
            }
        }
        if let Some(wdb) = pred.white_dark_bishops {
            if (white_bishops & Bitboard::DARK_SQUARES).count() != wdb {
                return false;
            }
        }
        if let Some(blb) = pred.black_light_bishops {
            if (black_bishops & Bitboard::LIGHT_SQUARES).count() != blb {
                return false;
            }
        }
        if let Some(bdb) = pred.black_dark_bishops {
            if (black_bishops & Bitboard::DARK_SQUARES).count() != bdb {
                return false;
            }
        }
        if let Some(lb) = pred.light_bishops {
            if (all_bishops & Bitboard::LIGHT_SQUARES).count() != lb {
                return false;
            }
        }
        if let Some(db) = pred.dark_bishops {
            if (all_bishops & Bitboard::DARK_SQUARES).count() != db {
                return false;
            }
        }

        if let Some(opposite) = pred.opposite_bishops {
            let is_opposite = check_opposite_bishops(white_bishops, black_bishops);
            if is_opposite != opposite {
                return false;
            }
        }

        if let Some(same) = pred.same_colored_bishops {
            let is_same = check_same_colored_bishops(white_bishops, black_bishops);
            if is_same != same {
                return false;
            }
        }

        true
    }

    /// Evaluate piece power and material point totals
    pub fn matches_power(pred: &super::query::PowerPredicate, pos: &Chess) -> bool {
        let board = pos.board();
        let w_power = calculate_power(board, Color::White);
        let b_power = calculate_power(board, Color::Black);

        match pred {
            super::query::PowerPredicate::WhitePower { op, value } => {
                compare_i32(w_power, *value, *op)
            }
            super::query::PowerPredicate::BlackPower { op, value } => {
                compare_i32(b_power, *value, *op)
            }
            super::query::PowerPredicate::TotalPower { op, value } => {
                compare_i32(w_power + b_power, *value, *op)
            }
            super::query::PowerPredicate::WhiteVsBlackPower { op } => {
                compare_i32(w_power, b_power, *op)
            }
            super::query::PowerPredicate::PowerDifference {
                op,
                value,
                absolute,
            } => {
                let diff = if *absolute {
                    (w_power - b_power).abs()
                } else {
                    w_power - b_power
                };
                compare_i32(diff, *value, *op)
            }
        }
    }
}

fn match_square_content(pos: &Chess, sq: Square, content: &SquareContent) -> bool {
    let piece_opt = pos.board().piece_at(sq);
    match content {
        SquareContent::Empty => piece_opt.is_none(),
        SquareContent::Occupied => piece_opt.is_some(),
        SquareContent::Piece(p) => piece_opt == Some(*p),
        SquareContent::Role(r) => piece_opt.map(|p| p.role) == Some(*r),
        SquareContent::Color(c) => piece_opt.map(|p| p.color) == Some(*c),
        SquareContent::AnyOf(pieces) => piece_opt.map(|p| pieces.contains(&p)).unwrap_or(false),
        SquareContent::NoneOf(pieces) => piece_opt.map(|p| !pieces.contains(&p)).unwrap_or(true),
    }
}

fn count_square_content(
    board: &shakmaty::Board,
    squares: Option<&[Square]>,
    content: &SquareContent,
) -> usize {
    let mask = match squares {
        Some(sqs) => {
            let mut bb = Bitboard::EMPTY;
            for &s in sqs {
                bb = bb.with(s);
            }
            bb
        }
        None => !Bitboard::EMPTY,
    };

    match content {
        SquareContent::Empty => (!board.occupied() & mask).count(),
        SquareContent::Occupied => (board.occupied() & mask).count(),
        SquareContent::Piece(p) => (board.by_piece(*p) & mask).count(),
        SquareContent::Role(r) => (board.by_role(*r) & mask).count(),
        SquareContent::Color(c) => (board.by_color(*c) & mask).count(),
        SquareContent::AnyOf(pieces) => {
            let mut union_bb = Bitboard::EMPTY;
            for &p in pieces {
                union_bb |= board.by_piece(p);
            }
            (union_bb & mask).count()
        }
        SquareContent::NoneOf(pieces) => {
            let mut union_bb = Bitboard::EMPTY;
            for &p in pieces {
                union_bb |= board.by_piece(p);
            }
            ((!union_bb) & mask).count()
        }
    }
}

/// Calculate standard piece power material total for a color (P=1, N=3, B=3, R=5, Q=9)
pub fn calculate_power(board: &shakmaty::Board, color: Color) -> i32 {
    let pawns = board
        .by_piece(Piece {
            color,
            role: Role::Pawn,
        })
        .count() as i32;
    let knights = board
        .by_piece(Piece {
            color,
            role: Role::Knight,
        })
        .count() as i32;
    let bishops = board
        .by_piece(Piece {
            color,
            role: Role::Bishop,
        })
        .count() as i32;
    let rooks = board
        .by_piece(Piece {
            color,
            role: Role::Rook,
        })
        .count() as i32;
    let queens = board
        .by_piece(Piece {
            color,
            role: Role::Queen,
        })
        .count() as i32;

    pawns + (knights * 3) + (bishops * 3) + (rooks * 5) + (queens * 9)
}

fn check_opposite_bishops(white_bishops: Bitboard, black_bishops: Bitboard) -> bool {
    let w_light = (white_bishops & Bitboard::LIGHT_SQUARES).count();
    let w_dark = (white_bishops & Bitboard::DARK_SQUARES).count();
    let b_light = (black_bishops & Bitboard::LIGHT_SQUARES).count();
    let b_dark = (black_bishops & Bitboard::DARK_SQUARES).count();

    (w_light == 1 && w_dark == 0 && b_dark == 1 && b_light == 0)
        || (w_dark == 1 && w_light == 0 && b_light == 1 && b_dark == 0)
}

fn check_same_colored_bishops(white_bishops: Bitboard, black_bishops: Bitboard) -> bool {
    let w_light = (white_bishops & Bitboard::LIGHT_SQUARES).count();
    let w_dark = (white_bishops & Bitboard::DARK_SQUARES).count();
    let b_light = (black_bishops & Bitboard::LIGHT_SQUARES).count();
    let b_dark = (black_bishops & Bitboard::DARK_SQUARES).count();

    (w_light == 1 && w_dark == 0 && b_light == 1 && b_dark == 0)
        || (w_dark == 1 && w_light == 0 && b_dark == 1 && b_light == 0)
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

fn compare_i32(actual: i32, expected: i32, op: ComparisonOp) -> bool {
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

pub fn match_wildcard_fen(pos: &Chess, pattern_str: &str) -> bool {
    let pattern_clean = pattern_str.trim();
    let board = pos.board();

    let mut parts = pattern_clean.split_whitespace();
    let placement_pattern = match parts.next() {
        Some(p) => p,
        None => return true,
    };

    let rank_patterns: Vec<&str> = placement_pattern.split('/').collect();
    if rank_patterns.len() > 8 {
        return false;
    }

    for (idx, &r_pat) in rank_patterns.iter().enumerate() {
        if r_pat == "*" || r_pat.is_empty() {
            continue;
        }

        let rank_idx = 7 - idx;
        let rank = shakmaty::Rank::new(rank_idx as u32);
        let mut actual_rank_pieces = [None; 8];
        for f in 0..8 {
            let file = shakmaty::File::new(f as u32);
            let sq = Square::from_coords(file, rank);
            actual_rank_pieces[f as usize] = board.piece_at(sq);
        }

        if !match_rank_pattern(r_pat, &actual_rank_pieces) {
            return false;
        }
    }

    if let Some(turn_part) = parts.next() {
        if turn_part != "*" && turn_part != "?" {
            let expected_turn = match turn_part.to_lowercase().as_str() {
                "w" | "white" => Some(Color::White),
                "b" | "black" => Some(Color::Black),
                _ => None,
            };
            if let Some(c) = expected_turn {
                if pos.turn() != c {
                    return false;
                }
            }
        }
    }

    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankCellMatcher {
    Empty,
    Piece(Piece),
    AnyWhite,
    AnyBlack,
    Any,
    Star,
}

fn match_rank_pattern(pat: &str, actual: &[Option<Piece>; 8]) -> bool {
    let mut pattern_cells = Vec::new();

    for ch in pat.chars() {
        if ch.is_ascii_digit() {
            let count = ch.to_digit(10).unwrap_or(0) as usize;
            for _ in 0..count {
                pattern_cells.push(RankCellMatcher::Empty);
            }
        } else if ch == '?' || ch == '.' {
            pattern_cells.push(RankCellMatcher::Any);
        } else if ch == '*' {
            pattern_cells.push(RankCellMatcher::Star);
        } else if ch == 'A' {
            pattern_cells.push(RankCellMatcher::AnyWhite);
        } else if ch == 'a' {
            pattern_cells.push(RankCellMatcher::AnyBlack);
        } else if let Some(piece) = parse_fen_piece_char(ch) {
            pattern_cells.push(RankCellMatcher::Piece(piece));
        }
    }

    match_cells_recursive(&pattern_cells, 0, actual, 0)
}

fn parse_fen_piece_char(ch: char) -> Option<Piece> {
    match ch {
        'P' => Some(Piece {
            color: Color::White,
            role: Role::Pawn,
        }),
        'N' => Some(Piece {
            color: Color::White,
            role: Role::Knight,
        }),
        'B' => Some(Piece {
            color: Color::White,
            role: Role::Bishop,
        }),
        'R' => Some(Piece {
            color: Color::White,
            role: Role::Rook,
        }),
        'Q' => Some(Piece {
            color: Color::White,
            role: Role::Queen,
        }),
        'K' => Some(Piece {
            color: Color::White,
            role: Role::King,
        }),
        'p' => Some(Piece {
            color: Color::Black,
            role: Role::Pawn,
        }),
        'n' => Some(Piece {
            color: Color::Black,
            role: Role::Knight,
        }),
        'b' => Some(Piece {
            color: Color::Black,
            role: Role::Bishop,
        }),
        'r' => Some(Piece {
            color: Color::Black,
            role: Role::Rook,
        }),
        'q' => Some(Piece {
            color: Color::Black,
            role: Role::Queen,
        }),
        'k' => Some(Piece {
            color: Color::Black,
            role: Role::King,
        }),
        _ => None,
    }
}

fn match_cells_recursive(
    pat: &[RankCellMatcher],
    pat_idx: usize,
    actual: &[Option<Piece>; 8],
    act_idx: usize,
) -> bool {
    if pat_idx == pat.len() && act_idx == 8 {
        return true;
    }
    if pat_idx == pat.len() {
        return false;
    }

    match pat[pat_idx] {
        RankCellMatcher::Star => {
            for skip in 0..=(8 - act_idx) {
                if match_cells_recursive(pat, pat_idx + 1, actual, act_idx + skip) {
                    return true;
                }
            }
            false
        }
        RankCellMatcher::Empty => {
            if act_idx < 8 && actual[act_idx].is_none() {
                match_cells_recursive(pat, pat_idx + 1, actual, act_idx + 1)
            } else {
                false
            }
        }
        RankCellMatcher::Piece(p) => {
            if act_idx < 8 && actual[act_idx] == Some(p) {
                match_cells_recursive(pat, pat_idx + 1, actual, act_idx + 1)
            } else {
                false
            }
        }
        RankCellMatcher::AnyWhite => {
            if act_idx < 8
                && actual[act_idx]
                    .map(|p| p.color == Color::White)
                    .unwrap_or(false)
            {
                match_cells_recursive(pat, pat_idx + 1, actual, act_idx + 1)
            } else {
                false
            }
        }
        RankCellMatcher::AnyBlack => {
            if act_idx < 8
                && actual[act_idx]
                    .map(|p| p.color == Color::Black)
                    .unwrap_or(false)
            {
                match_cells_recursive(pat, pat_idx + 1, actual, act_idx + 1)
            } else {
                false
            }
        }
        RankCellMatcher::Any => {
            if act_idx < 8 {
                match_cells_recursive(pat, pat_idx + 1, actual, act_idx + 1)
            } else {
                false
            }
        }
    }
}
