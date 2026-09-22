use shakmaty::{Bitboard, Chess, File, Piece, Rank, Square};
use std::collections::HashMap;

use super::query::{
    MaterialPredicate, PawnPredicate, PieceMatcher, PositionPattern, SearchQuery, SetPredicate,
    SquareContent, SquareOrPiece, SquareSetExpr, TacticalPredicate,
};

/// Board and geometric transformation symmetries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoardSymmetry {
    /// No transformation (identity)
    Identity,
    /// Horizontal flip (left-right file mirror: a <-> h, b <-> g, c <-> f, d <-> e)
    HorizontalMirror,
    /// Vertical flip (rank mirror: 1 <-> 8, 2 <-> 7, etc.)
    VerticalMirror,
    /// 180-degree board rotation (both horizontal and vertical)
    Rotate180,
    /// 90-degree clockwise board rotation
    Rotate90,
    /// 270-degree clockwise board rotation
    Rotate270,
    /// All 4 rotational orientations (0, 90, 180, 270)
    AllRotations,
    /// Color inversion (invert piece colors and rank mirror, preserving turn perspective)
    ColorInvert,
    /// Color inversion with horizontal mirror
    ColorInvertHorizontal,
    /// Any of the 4 spatial transformations (Identity, Horizontal, Vertical, Rotate180)
    AnySpatialSymmetry,
    /// Any of the 8 total symmetries (spatial + color inverted)
    AnyTotalSymmetry,
}

impl BoardSymmetry {
    /// Get all concrete symmetries represented by this symmetry mode
    pub fn expand(&self) -> Vec<BoardSymmetry> {
        match self {
            BoardSymmetry::Identity => vec![BoardSymmetry::Identity],
            BoardSymmetry::HorizontalMirror => vec![BoardSymmetry::HorizontalMirror],
            BoardSymmetry::VerticalMirror => vec![BoardSymmetry::VerticalMirror],
            BoardSymmetry::Rotate180 => vec![BoardSymmetry::Rotate180],
            BoardSymmetry::Rotate90 => vec![BoardSymmetry::Rotate90],
            BoardSymmetry::Rotate270 => vec![BoardSymmetry::Rotate270],
            BoardSymmetry::AllRotations => vec![
                BoardSymmetry::Identity,
                BoardSymmetry::Rotate90,
                BoardSymmetry::Rotate180,
                BoardSymmetry::Rotate270,
            ],
            BoardSymmetry::ColorInvert => vec![BoardSymmetry::Identity, BoardSymmetry::ColorInvert],
            BoardSymmetry::ColorInvertHorizontal => vec![
                BoardSymmetry::Identity,
                BoardSymmetry::ColorInvertHorizontal,
            ],
            BoardSymmetry::AnySpatialSymmetry => vec![
                BoardSymmetry::Identity,
                BoardSymmetry::HorizontalMirror,
                BoardSymmetry::VerticalMirror,
                BoardSymmetry::Rotate180,
                BoardSymmetry::Rotate90,
                BoardSymmetry::Rotate270,
            ],
            BoardSymmetry::AnyTotalSymmetry => vec![
                BoardSymmetry::Identity,
                BoardSymmetry::HorizontalMirror,
                BoardSymmetry::VerticalMirror,
                BoardSymmetry::Rotate180,
                BoardSymmetry::Rotate90,
                BoardSymmetry::Rotate270,
                BoardSymmetry::ColorInvert,
                BoardSymmetry::ColorInvertHorizontal,
            ],
        }
    }

    /// Transform a square under this symmetry
    pub fn transform_square(&self, sq: Square) -> Square {
        let f = sq.file() as u32;
        let r = sq.rank() as u32;

        let (new_f, new_r) = match self {
            BoardSymmetry::Identity => (f, r),
            BoardSymmetry::HorizontalMirror => (7 - f, r),
            BoardSymmetry::VerticalMirror => (f, 7 - r),
            BoardSymmetry::Rotate180 => (7 - f, 7 - r),
            BoardSymmetry::Rotate90 => (r, 7 - f),
            BoardSymmetry::Rotate270 => (7 - r, f),
            BoardSymmetry::ColorInvert => (f, 7 - r),
            BoardSymmetry::ColorInvertHorizontal => (7 - f, 7 - r),
            BoardSymmetry::AnySpatialSymmetry
            | BoardSymmetry::AnyTotalSymmetry
            | BoardSymmetry::AllRotations => (f, r),
        };

        Square::from_coords(File::new(new_f), Rank::new(new_r))
    }

    /// Transform a piece under this symmetry (inverting color if color inversion mode)
    pub fn transform_piece(&self, piece: Piece) -> Piece {
        match self {
            BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => Piece {
                color: piece.color.other(),
                role: piece.role,
            },
            _ => piece,
        }
    }

    /// Transform a piece matcher
    pub fn transform_piece_matcher(&self, pm: &PieceMatcher) -> PieceMatcher {
        let new_color = match self {
            BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                pm.color.map(|c| c.other())
            }
            _ => pm.color,
        };
        PieceMatcher {
            color: new_color,
            role: pm.role,
        }
    }

    /// Transform a square or piece specifier
    pub fn transform_square_or_piece(&self, target: &SquareOrPiece) -> SquareOrPiece {
        match target {
            SquareOrPiece::Square(sq) => SquareOrPiece::Square(self.transform_square(*sq)),
            SquareOrPiece::Piece(pm) => SquareOrPiece::Piece(self.transform_piece_matcher(pm)),
            SquareOrPiece::Variable(v) => SquareOrPiece::Variable(v.clone()),
            SquareOrPiece::Empty => SquareOrPiece::Empty,
        }
    }

    /// Transform a square content requirement
    pub fn transform_square_content(&self, content: &SquareContent) -> SquareContent {
        match content {
            SquareContent::Empty => SquareContent::Empty,
            SquareContent::Occupied => SquareContent::Occupied,
            SquareContent::Piece(p) => SquareContent::Piece(self.transform_piece(*p)),
            SquareContent::Role(r) => SquareContent::Role(*r),
            SquareContent::Color(c) => match self {
                BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                    SquareContent::Color(c.other())
                }
                _ => SquareContent::Color(*c),
            },
            SquareContent::AnyOf(pieces) => {
                SquareContent::AnyOf(pieces.iter().map(|p| self.transform_piece(*p)).collect())
            }
            SquareContent::NoneOf(pieces) => {
                SquareContent::NoneOf(pieces.iter().map(|p| self.transform_piece(*p)).collect())
            }
        }
    }

    /// Transform a FEN string under this board symmetry
    pub fn transform_fen(&self, fen_str: &str) -> Result<String, String> {
        let parts: Vec<&str> = fen_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty FEN string".to_string());
        }

        let piece_placement = parts[0];
        let transformed_placement = self.transform_piece_placement(piece_placement)?;

        if parts.len() == 1 {
            return Ok(transformed_placement);
        }

        let active_color = parts.get(1).copied().unwrap_or("w");
        let transformed_active = match self {
            BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                if active_color == "w" {
                    "b"
                } else {
                    "w"
                }
            }
            _ => active_color,
        };

        let castling = parts.get(2).copied().unwrap_or("-");
        let transformed_castling = self.transform_castling_rights(castling);

        let ep = parts.get(3).copied().unwrap_or("-");
        let transformed_ep = if ep != "-" {
            if let Ok(sq) = ep.parse::<Square>() {
                let new_sq = self.transform_square(sq);
                format!("{new_sq}")
            } else {
                ep.to_string()
            }
        } else {
            "-".to_string()
        };

        let halfmove = parts.get(4).copied().unwrap_or("0");
        let fullmove = parts.get(5).copied().unwrap_or("1");

        Ok(format!(
            "{transformed_placement} {transformed_active} {transformed_castling} {transformed_ep} {halfmove} {fullmove}"
        ))
    }

    /// Transform piece placement (the 8 rank segments separated by '/')
    pub fn transform_piece_placement(&self, placement: &str) -> Result<String, String> {
        let ranks: Vec<&str> = placement.split('/').collect();
        if ranks.len() != 8 {
            return Err(format!(
                "Invalid FEN placement ranks count: {}",
                ranks.len()
            ));
        }

        let mut transformed_ranks = Vec::with_capacity(8);
        for rank_str in ranks {
            let mut transformed_rank = String::new();
            for ch in rank_str.chars() {
                if ch.is_ascii_digit() {
                    transformed_rank.push(ch);
                } else {
                    let new_ch = match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            if ch.is_ascii_uppercase() {
                                ch.to_ascii_lowercase()
                            } else {
                                ch.to_ascii_uppercase()
                            }
                        }
                        _ => ch,
                    };
                    transformed_rank.push(new_ch);
                }
            }
            transformed_ranks.push(transformed_rank);
        }

        // Horizontal flip: reverse files inside each rank
        if matches!(
            self,
            BoardSymmetry::HorizontalMirror
                | BoardSymmetry::Rotate180
                | BoardSymmetry::ColorInvertHorizontal
        ) {
            for rank in &mut transformed_ranks {
                *rank = rank.chars().rev().collect();
            }
        }

        // Vertical flip / ColorInvert: reverse rank order (Rank 8 <-> Rank 1)
        if matches!(
            self,
            BoardSymmetry::VerticalMirror
                | BoardSymmetry::Rotate180
                | BoardSymmetry::ColorInvert
                | BoardSymmetry::ColorInvertHorizontal
        ) {
            transformed_ranks.reverse();
        }

        Ok(transformed_ranks.join("/"))
    }

    fn transform_castling_rights(&self, castling: &str) -> String {
        if castling == "-" {
            return "-".to_string();
        }

        let mut k_w = castling.contains('K');
        let mut q_w = castling.contains('Q');
        let mut k_b = castling.contains('k');
        let mut q_b = castling.contains('q');

        // Color invert swaps White and Black castling rights
        if matches!(
            self,
            BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal
        ) {
            std::mem::swap(&mut k_w, &mut k_b);
            std::mem::swap(&mut q_w, &mut q_b);
        }

        // Horizontal mirror swaps Kingside and Queenside castling rights
        if matches!(
            self,
            BoardSymmetry::HorizontalMirror
                | BoardSymmetry::Rotate180
                | BoardSymmetry::ColorInvertHorizontal
        ) {
            std::mem::swap(&mut k_w, &mut q_w);
            std::mem::swap(&mut k_b, &mut q_b);
        }

        let mut res = String::new();
        if k_w {
            res.push('K');
        }
        if q_w {
            res.push('Q');
        }
        if k_b {
            res.push('k');
        }
        if q_b {
            res.push('q');
        }

        if res.is_empty() {
            "-".to_string()
        } else {
            res
        }
    }

    /// Transform an entire position pattern under this symmetry
    pub fn transform_position_pattern(&self, pattern: &PositionPattern) -> PositionPattern {
        match pattern {
            PositionPattern::ExactFen(fen) => {
                if let Ok(transformed) = self.transform_fen(fen) {
                    PositionPattern::ExactFen(transformed)
                } else {
                    PositionPattern::ExactFen(fen.clone())
                }
            }
            PositionPattern::PiecePlacement(placement) => {
                if let Ok(transformed) = self.transform_piece_placement(placement) {
                    PositionPattern::PiecePlacement(transformed)
                } else {
                    PositionPattern::PiecePlacement(placement.clone())
                }
            }
            PositionPattern::Squares(square_map) => {
                let mut new_map = HashMap::new();
                for (&sq, content) in square_map {
                    let new_sq = self.transform_square(sq);
                    let new_content = self.transform_square_content(content);
                    new_map.insert(new_sq, new_content);
                }
                PositionPattern::Squares(new_map)
            }
            PositionPattern::PieceCount {
                content,
                squares,
                op,
                count,
            } => PositionPattern::PieceCount {
                content: self.transform_square_content(content),
                squares: squares
                    .as_ref()
                    .map(|sqs| sqs.iter().map(|&s| self.transform_square(s)).collect()),
                op: *op,
                count: *count,
            },
            PositionPattern::Turn(color) => match self {
                BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                    PositionPattern::Turn(color.other())
                }
                _ => PositionPattern::Turn(*color),
            },
            PositionPattern::Attack { from, to } => PositionPattern::Attack {
                from: self.transform_square(*from),
                to: self.transform_square(*to),
            },
            PositionPattern::IsAttacked { square, by_color } => match self {
                BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                    PositionPattern::IsAttacked {
                        square: self.transform_square(*square),
                        by_color: by_color.other(),
                    }
                }
                _ => PositionPattern::IsAttacked {
                    square: self.transform_square(*square),
                    by_color: *by_color,
                },
            },
            PositionPattern::MultiSquare { content, squares } => PositionPattern::MultiSquare {
                content: self.transform_square_content(content),
                squares: squares
                    .iter()
                    .map(|sq| self.transform_square(*sq))
                    .collect(),
            },
            PositionPattern::VariableSquareFilter { var_name, squares } => {
                PositionPattern::VariableSquareFilter {
                    var_name: var_name.clone(),
                    squares: squares.iter().map(|s| self.transform_square(*s)).collect(),
                }
            }
            other => other.clone(),
        }
    }

    /// Transform a tactical predicate under this symmetry
    pub fn transform_tactical_predicate(&self, pred: &TacticalPredicate) -> TacticalPredicate {
        match pred {
            TacticalPredicate::Pin {
                pinners,
                pinneds,
                targets,
            } => TacticalPredicate::Pin {
                pinners: pinners
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
                pinneds: pinneds
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
                targets: targets
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
            },
            TacticalPredicate::Fork {
                attackers,
                target_slots,
                targets_pool,
                min_targets,
            } => TacticalPredicate::Fork {
                attackers: attackers
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
                target_slots: target_slots
                    .iter()
                    .map(|slot| {
                        slot.iter()
                            .map(|p| self.transform_piece_matcher(p))
                            .collect()
                    })
                    .collect(),
                targets_pool: targets_pool
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
                min_targets: *min_targets,
            },
            TacticalPredicate::DiscoveredAttack { color, is_check } => {
                TacticalPredicate::DiscoveredAttack {
                    color: match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            color.other()
                        }
                        _ => *color,
                    },
                    is_check: *is_check,
                }
            }
            TacticalPredicate::Skewer {
                attackers,
                fronts,
                rears,
            } => TacticalPredicate::Skewer {
                attackers: attackers
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
                fronts: fronts
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
                rears: rears
                    .iter()
                    .map(|p| self.transform_piece_matcher(p))
                    .collect(),
            },
            TacticalPredicate::TrappedPiece { piece } => TacticalPredicate::TrappedPiece {
                piece: self.transform_piece_matcher(piece),
            },
            TacticalPredicate::Outpost { piece, square } => TacticalPredicate::Outpost {
                piece: self.transform_piece_matcher(piece),
                square: square.map(|sq| self.transform_square(sq)),
            },
            TacticalPredicate::RookOnSeventh { color } => TacticalPredicate::RookOnSeventh {
                color: match self {
                    BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                        color.other()
                    }
                    _ => *color,
                },
            },
            TacticalPredicate::OpenFile {
                file,
                semi_open_for,
            } => TacticalPredicate::OpenFile {
                file: file.map(|f| match self {
                    BoardSymmetry::HorizontalMirror
                    | BoardSymmetry::Rotate180
                    | BoardSymmetry::ColorInvertHorizontal => File::new(7 - f as u32),
                    _ => f,
                }),
                semi_open_for: semi_open_for.map(|c| match self {
                    BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => c.other(),
                    _ => c,
                }),
            },
            TacticalPredicate::Distance {
                sq1,
                sq2,
                op,
                distance,
            } => TacticalPredicate::Distance {
                sq1: self.transform_square_or_piece(sq1),
                sq2: self.transform_square_or_piece(sq2),
                op: *op,
                distance: *distance,
            },
            TacticalPredicate::Attacks { attacker, target } => TacticalPredicate::Attacks {
                attacker: self.transform_square_or_piece(attacker),
                target: self.transform_square_or_piece(target),
            },
        }
    }

    /// Transform a material predicate
    pub fn transform_material_predicate(&self, pred: &MaterialPredicate) -> MaterialPredicate {
        match self {
            BoardSymmetry::ColorInvert => MaterialPredicate {
                white_pawns: pred.black_pawns,
                white_knights: pred.black_knights,
                white_bishops: pred.black_bishops,
                white_light_bishops: pred.black_dark_bishops,
                white_dark_bishops: pred.black_light_bishops,
                white_rooks: pred.black_rooks,
                white_queens: pred.black_queens,
                black_pawns: pred.white_pawns,
                black_knights: pred.white_knights,
                black_bishops: pred.white_bishops,
                black_light_bishops: pred.white_dark_bishops,
                black_dark_bishops: pred.white_light_bishops,
                light_bishops: pred.dark_bishops,
                dark_bishops: pred.light_bishops,
                black_rooks: pred.white_rooks,
                black_queens: pred.white_queens,
                material_difference: pred.material_difference.map(|(op, d)| (op.invert(), -d)),
                opposite_bishops: pred.opposite_bishops,
                same_colored_bishops: pred.same_colored_bishops,
            },
            BoardSymmetry::ColorInvertHorizontal => MaterialPredicate {
                white_pawns: pred.black_pawns,
                white_knights: pred.black_knights,
                white_bishops: pred.black_bishops,
                white_light_bishops: pred.black_light_bishops,
                white_dark_bishops: pred.black_dark_bishops,
                white_rooks: pred.black_rooks,
                white_queens: pred.black_queens,
                black_pawns: pred.white_pawns,
                black_knights: pred.white_knights,
                black_bishops: pred.white_bishops,
                black_light_bishops: pred.white_light_bishops,
                black_dark_bishops: pred.white_dark_bishops,
                light_bishops: pred.light_bishops,
                dark_bishops: pred.dark_bishops,
                black_rooks: pred.white_rooks,
                black_queens: pred.white_queens,
                material_difference: pred.material_difference.map(|(op, d)| (op.invert(), -d)),
                opposite_bishops: pred.opposite_bishops,
                same_colored_bishops: pred.same_colored_bishops,
            },
            BoardSymmetry::HorizontalMirror | BoardSymmetry::VerticalMirror => MaterialPredicate {
                white_pawns: pred.white_pawns,
                white_knights: pred.white_knights,
                white_bishops: pred.white_bishops,
                white_light_bishops: pred.white_dark_bishops,
                white_dark_bishops: pred.white_light_bishops,
                white_rooks: pred.white_rooks,
                white_queens: pred.white_queens,
                black_pawns: pred.black_pawns,
                black_knights: pred.black_knights,
                black_bishops: pred.black_bishops,
                black_light_bishops: pred.black_dark_bishops,
                black_dark_bishops: pred.black_light_bishops,
                light_bishops: pred.dark_bishops,
                dark_bishops: pred.light_bishops,
                black_rooks: pred.black_rooks,
                black_queens: pred.black_queens,
                material_difference: pred.material_difference,
                opposite_bishops: pred.opposite_bishops,
                same_colored_bishops: pred.same_colored_bishops,
            },
            _ => pred.clone(),
        }
    }

    /// Transform an entire search query
    pub fn transform_query(&self, query: &SearchQuery) -> SearchQuery {
        match query {
            SearchQuery::Position(pat) => {
                SearchQuery::Position(self.transform_position_pattern(pat))
            }
            SearchQuery::Tactical(tactical) => {
                SearchQuery::Tactical(self.transform_tactical_predicate(tactical))
            }
            SearchQuery::Material(mat) => {
                SearchQuery::Material(self.transform_material_predicate(mat))
            }
            SearchQuery::Pawn(pawn) => SearchQuery::Pawn(match pawn {
                PawnPredicate::PassedPawns { color, op, count } => PawnPredicate::PassedPawns {
                    color: match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            color.other()
                        }
                        _ => *color,
                    },
                    op: *op,
                    count: *count,
                },
                PawnPredicate::IsolatedPawns { color, op, count } => PawnPredicate::IsolatedPawns {
                    color: match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            color.other()
                        }
                        _ => *color,
                    },
                    op: *op,
                    count: *count,
                },
                PawnPredicate::DoubledPawns { color, op, count } => PawnPredicate::DoubledPawns {
                    color: match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            color.other()
                        }
                        _ => *color,
                    },
                    op: *op,
                    count: *count,
                },
                PawnPredicate::BackwardPawns { color, op, count } => PawnPredicate::BackwardPawns {
                    color: match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            color.other()
                        }
                        _ => *color,
                    },
                    op: *op,
                    count: *count,
                },
                PawnPredicate::PawnIslands { color, op, count } => PawnPredicate::PawnIslands {
                    color: match self {
                        BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                            color.other()
                        }
                        _ => *color,
                    },
                    op: *op,
                    count: *count,
                },
            }),
            SearchQuery::And(subs) => {
                SearchQuery::And(subs.iter().map(|s| self.transform_query(s)).collect())
            }
            SearchQuery::Or(subs) => {
                SearchQuery::Or(subs.iter().map(|s| self.transform_query(s)).collect())
            }
            SearchQuery::Not(sub) => SearchQuery::Not(Box::new(self.transform_query(sub))),
            SearchQuery::Parent(sub) => SearchQuery::Parent(Box::new(self.transform_query(sub))),
            SearchQuery::Child(sub) => SearchQuery::Child(Box::new(self.transform_query(sub))),
            SearchQuery::Play {
                move_pattern,
                outcome_query,
            } => SearchQuery::Play {
                move_pattern: self.transform_move_pattern(move_pattern),
                outcome_query: Box::new(self.transform_query(outcome_query)),
            },
            SearchQuery::PlyRange { range, query: sub } => SearchQuery::PlyRange {
                range: range.clone(),
                query: Box::new(self.transform_query(sub)),
            },
            SearchQuery::Occurrences {
                min,
                max,
                query: sub,
            } => SearchQuery::Occurrences {
                min: *min,
                max: *max,
                query: Box::new(self.transform_query(sub)),
            },
            SearchQuery::Symmetric {
                query: sub,
                symmetry,
            } => SearchQuery::Symmetric {
                query: Box::new(self.transform_query(sub)),
                symmetry: *symmetry,
            },
            SearchQuery::Header(h) => SearchQuery::Header(self.transform_header_predicate(h)),
            SearchQuery::Move(mv) => SearchQuery::Move(self.transform_move_pattern(mv)),
            SearchQuery::Path(path_pattern) => {
                let mut new_path = path_pattern.clone();
                new_path.moves = path_pattern
                    .moves
                    .iter()
                    .map(|m| self.transform_move_pattern(m))
                    .collect();
                new_path.steps = path_pattern
                    .steps
                    .iter()
                    .map(|s| match s {
                        super::query::PathStep::Move {
                            pattern,
                            repeat_min,
                            repeat_max,
                        } => super::query::PathStep::Move {
                            pattern: self.transform_move_pattern(pattern),
                            repeat_min: *repeat_min,
                            repeat_max: *repeat_max,
                        },
                        super::query::PathStep::Gap { min, max } => super::query::PathStep::Gap {
                            min: *min,
                            max: *max,
                        },
                    })
                    .collect();
                SearchQuery::Path(new_path)
            }
            SearchQuery::Power(pow) => SearchQuery::Power(match self {
                BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => match pow {
                    super::query::PowerPredicate::WhitePower { op, value } => {
                        super::query::PowerPredicate::BlackPower {
                            op: *op,
                            value: *value,
                        }
                    }
                    super::query::PowerPredicate::BlackPower { op, value } => {
                        super::query::PowerPredicate::WhitePower {
                            op: *op,
                            value: *value,
                        }
                    }
                    super::query::PowerPredicate::WhiteVsBlackPower { op } => {
                        super::query::PowerPredicate::WhiteVsBlackPower { op: op.invert() }
                    }
                    super::query::PowerPredicate::PowerDifference {
                        op,
                        value,
                        absolute,
                    } => super::query::PowerPredicate::PowerDifference {
                        op: if *absolute { *op } else { op.invert() },
                        value: if *absolute { *value } else { -value },
                        absolute: *absolute,
                    },
                    other => other.clone(),
                },
                _ => pow.clone(),
            }),
            SearchQuery::VariableBinding {
                var_name,
                domain,
                query: sub,
            } => SearchQuery::VariableBinding {
                var_name: var_name.clone(),
                domain: match domain {
                    super::query::VariableDomain::Piece(pm) => {
                        super::query::VariableDomain::Piece(self.transform_piece_matcher(pm))
                    }
                    super::query::VariableDomain::SquareSet(sqs) => {
                        super::query::VariableDomain::SquareSet(
                            sqs.iter().map(|s| self.transform_square(*s)).collect(),
                        )
                    }
                    super::query::VariableDomain::AnyPiece => {
                        super::query::VariableDomain::AnyPiece
                    }
                },
                query: Box::new(self.transform_query(sub)),
            },
            SearchQuery::SquareSet(set_pred) => {
                SearchQuery::SquareSet(self.transform_set_predicate(set_pred))
            }
            SearchQuery::CqlPath(cql_path) => {
                SearchQuery::CqlPath(self.transform_cql_path_pattern(cql_path))
            }
            SearchQuery::CqlLine(cql_line) => {
                SearchQuery::CqlLine(self.transform_cql_line_pattern(cql_line))
            }
            other => other.clone(),
        }
    }

    /// Transform a CqlLinePattern under geometric or color inversion symmetry
    pub fn transform_cql_line_pattern(
        &self,
        line: &super::query::CqlLinePattern,
    ) -> super::query::CqlLinePattern {
        super::query::CqlLinePattern {
            min_length: line.min_length,
            max_length: line.max_length,
            direction: line.direction,
            single_color: match line.single_color {
                Some(Some(c)) => match self {
                    BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                        Some(Some(c.other()))
                    }
                    _ => Some(Some(c)),
                },
                other => other,
            },
            first_match: line.first_match,
            last_position: line.last_position,
            nest_ban: line.nest_ban,
            primary_only: line.primary_only,
            start_ply_range: line.start_ply_range.clone(),
            constituents: line
                .constituents
                .iter()
                .map(|c| self.transform_cql_path_constituent(c))
                .collect(),
        }
    }

    /// Transform a CqlPathPattern under geometric or color inversion symmetry
    pub fn transform_cql_path_pattern(
        &self,
        path: &super::query::CqlPathPattern,
    ) -> super::query::CqlPathPattern {
        super::query::CqlPathPattern {
            constituents: path
                .constituents
                .iter()
                .map(|c| self.transform_cql_path_constituent(c))
                .collect(),
            single_color: match path.single_color {
                Some(Some(c)) => match self {
                    BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => {
                        Some(Some(c.other()))
                    }
                    _ => Some(Some(c)),
                },
                other => other,
            },
            start_ply_range: path.start_ply_range.clone(),
        }
    }

    /// Transform a CqlPathConstituent under geometric or color inversion symmetry
    pub fn transform_cql_path_constituent(
        &self,
        c: &super::query::CqlPathConstituent,
    ) -> super::query::CqlPathConstituent {
        match c {
            super::query::CqlPathConstituent::Move(m) => {
                super::query::CqlPathConstituent::Move(self.transform_move_pattern(m))
            }
            super::query::CqlPathConstituent::Filter(q) => {
                super::query::CqlPathConstituent::Filter(Box::new(self.transform_query(q)))
            }
            super::query::CqlPathConstituent::Repetition {
                constituent,
                min,
                max,
            } => super::query::CqlPathConstituent::Repetition {
                constituent: Box::new(self.transform_cql_path_constituent(constituent)),
                min: *min,
                max: *max,
            },
            super::query::CqlPathConstituent::Chain(subs) => {
                super::query::CqlPathConstituent::Chain(
                    subs.iter()
                        .map(|s| self.transform_cql_path_constituent(s))
                        .collect(),
                )
            }
        }
    }

    /// Transform a SetPredicate under geometric or color inversion symmetry
    pub fn transform_set_predicate(&self, pred: &SetPredicate) -> SetPredicate {
        match pred {
            SetPredicate::NonEmpty(expr) => {
                SetPredicate::NonEmpty(self.transform_square_set_expr(expr))
            }
            SetPredicate::CountComparison { expr, op, count } => SetPredicate::CountComparison {
                expr: self.transform_square_set_expr(expr),
                op: *op,
                count: *count,
            },
            SetPredicate::SetComparison { left, op, right } => SetPredicate::SetComparison {
                left: self.transform_square_set_expr(left),
                op: *op,
                right: self.transform_square_set_expr(right),
            },
        }
    }

    /// Transform a SquareSetExpr under geometric or color inversion symmetry
    pub fn transform_square_set_expr(&self, expr: &SquareSetExpr) -> SquareSetExpr {
        match expr {
            SquareSetExpr::Piece(content) => {
                SquareSetExpr::Piece(self.transform_square_content(content))
            }
            SquareSetExpr::Squares(bb) => {
                let mut new_bb = Bitboard::EMPTY;
                for sq in *bb {
                    new_bb.add(self.transform_square(sq));
                }
                SquareSetExpr::Squares(new_bb)
            }
            SquareSetExpr::Variable(var) => SquareSetExpr::Variable(var.clone()),
            SquareSetExpr::Intersection(left, right) => SquareSetExpr::Intersection(
                Box::new(self.transform_square_set_expr(left)),
                Box::new(self.transform_square_set_expr(right)),
            ),
            SquareSetExpr::Union(left, right) => SquareSetExpr::Union(
                Box::new(self.transform_square_set_expr(left)),
                Box::new(self.transform_square_set_expr(right)),
            ),
            SquareSetExpr::Difference(left, right) => SquareSetExpr::Difference(
                Box::new(self.transform_square_set_expr(left)),
                Box::new(self.transform_square_set_expr(right)),
            ),
            SquareSetExpr::Complement(inner) => {
                SquareSetExpr::Complement(Box::new(self.transform_square_set_expr(inner)))
            }
            SquareSetExpr::Attacks { attacker, target } => SquareSetExpr::Attacks {
                attacker: Box::new(self.transform_square_set_expr(attacker)),
                target: Box::new(self.transform_square_set_expr(target)),
            },
            SquareSetExpr::Attackers { attacker, target } => SquareSetExpr::Attackers {
                attacker: Box::new(self.transform_square_set_expr(attacker)),
                target: Box::new(self.transform_square_set_expr(target)),
            },
            SquareSetExpr::Ray { direction, origin } => SquareSetExpr::Ray {
                direction: self.transform_direction(*direction),
                origin: Box::new(self.transform_square_set_expr(origin)),
            },
            SquareSetExpr::Between { from, to } => SquareSetExpr::Between {
                from: Box::new(self.transform_square_set_expr(from)),
                to: Box::new(self.transform_square_set_expr(to)),
            },
            SquareSetExpr::Shift {
                direction,
                min_dist,
                max_dist,
                expr,
            } => SquareSetExpr::Shift {
                direction: self.transform_direction(*direction),
                min_dist: *min_dist,
                max_dist: *max_dist,
                expr: Box::new(self.transform_square_set_expr(expr)),
            },
        }
    }

    /// Transform a board direction under geometric symmetry
    pub fn transform_direction(
        &self,
        dir: crate::search::query::Direction,
    ) -> crate::search::query::Direction {
        use crate::search::query::Direction;
        match self {
            BoardSymmetry::Identity => dir,
            BoardSymmetry::HorizontalMirror => match dir {
                Direction::Up => Direction::Up,
                Direction::Down => Direction::Down,
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
                Direction::NorthEast => Direction::NorthWest,
                Direction::NorthWest => Direction::NorthEast,
                Direction::SouthEast => Direction::SouthWest,
                Direction::SouthWest => Direction::SouthEast,
                other => other,
            },
            BoardSymmetry::VerticalMirror | BoardSymmetry::ColorInvert => match dir {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
                Direction::Left => Direction::Left,
                Direction::Right => Direction::Right,
                Direction::NorthEast => Direction::SouthEast,
                Direction::NorthWest => Direction::SouthWest,
                Direction::SouthEast => Direction::NorthEast,
                Direction::SouthWest => Direction::NorthWest,
                other => other,
            },
            BoardSymmetry::Rotate180 | BoardSymmetry::ColorInvertHorizontal => match dir {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
                Direction::NorthEast => Direction::SouthWest,
                Direction::NorthWest => Direction::SouthEast,
                Direction::SouthEast => Direction::NorthWest,
                Direction::SouthWest => Direction::NorthEast,
                other => other,
            },
            BoardSymmetry::Rotate90 => match dir {
                Direction::Up => Direction::Right,
                Direction::Right => Direction::Down,
                Direction::Down => Direction::Left,
                Direction::Left => Direction::Up,
                Direction::NorthEast => Direction::SouthEast,
                Direction::SouthEast => Direction::SouthWest,
                Direction::SouthWest => Direction::NorthWest,
                Direction::NorthWest => Direction::NorthEast,
                Direction::Vertical => Direction::Horizontal,
                Direction::Horizontal => Direction::Vertical,
                other => other,
            },
            BoardSymmetry::Rotate270 => match dir {
                Direction::Up => Direction::Left,
                Direction::Left => Direction::Down,
                Direction::Down => Direction::Right,
                Direction::Right => Direction::Up,
                Direction::NorthEast => Direction::NorthWest,
                Direction::NorthWest => Direction::SouthWest,
                Direction::SouthWest => Direction::SouthEast,
                Direction::SouthEast => Direction::NorthEast,
                Direction::Vertical => Direction::Horizontal,
                Direction::Horizontal => Direction::Vertical,
                other => other,
            },
            _ => dir,
        }
    }

    /// Transform a header predicate under color inversion
    pub fn transform_header_predicate(
        &self,
        pred: &super::query::HeaderPredicate,
    ) -> super::query::HeaderPredicate {
        use super::query::HeaderPredicate;
        match self {
            BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal => match pred {
                HeaderPredicate::White {
                    name,
                    op,
                    case_sensitive,
                } => HeaderPredicate::Black {
                    name: name.clone(),
                    op: *op,
                    case_sensitive: *case_sensitive,
                },
                HeaderPredicate::Black {
                    name,
                    op,
                    case_sensitive,
                } => HeaderPredicate::White {
                    name: name.clone(),
                    op: *op,
                    case_sensitive: *case_sensitive,
                },
                HeaderPredicate::WhiteElo { op, value } => HeaderPredicate::BlackElo {
                    op: *op,
                    value: *value,
                },
                HeaderPredicate::BlackElo { op, value } => HeaderPredicate::WhiteElo {
                    op: *op,
                    value: *value,
                },
                HeaderPredicate::EloDiff {
                    op,
                    value,
                    absolute,
                } => HeaderPredicate::EloDiff {
                    op: if *absolute { *op } else { op.invert() },
                    value: if *absolute { *value } else { -value },
                    absolute: *absolute,
                },
                HeaderPredicate::Result { expected } => HeaderPredicate::Result {
                    expected: match expected.as_str() {
                        "1-0" => "0-1".to_string(),
                        "0-1" => "1-0".to_string(),
                        other => other.to_string(),
                    },
                },
                other => other.clone(),
            },
            _ => pred.clone(),
        }
    }

    /// Transform a move pattern under this symmetry
    pub fn transform_move_pattern(
        &self,
        pat: &super::query::MovePattern,
    ) -> super::query::MovePattern {
        let mut new_pat = pat.clone();
        if matches!(
            self,
            BoardSymmetry::ColorInvert | BoardSymmetry::ColorInvertHorizontal
        ) {
            new_pat.color = pat.color.map(|c| c.other());
        }
        if let Some(from_sq) = pat.from {
            new_pat.from = Some(self.transform_square(from_sq));
        }
        if let Some(ref from_sqs) = pat.from_squares {
            new_pat.from_squares =
                Some(from_sqs.iter().map(|s| self.transform_square(*s)).collect());
        }
        if let Some(ref from_pcs) = pat.from_pieces {
            new_pat.from_pieces = Some(
                from_pcs
                    .iter()
                    .map(|p| self.transform_square_content(p))
                    .collect(),
            );
        }
        if let Some(to_sq) = pat.to {
            new_pat.to = Some(self.transform_square(to_sq));
        }
        if let Some(ref to_sqs) = pat.to_squares {
            new_pat.to_squares = Some(to_sqs.iter().map(|s| self.transform_square(*s)).collect());
        }
        if let Some(ref to_pcs) = pat.to_pieces {
            new_pat.to_pieces = Some(
                to_pcs
                    .iter()
                    .map(|p| self.transform_square_content(p))
                    .collect(),
            );
        }
        if let Some(ref cap_pcs) = pat.captured_pieces {
            new_pat.captured_pieces = Some(
                cap_pcs
                    .iter()
                    .map(|p| self.transform_square_content(p))
                    .collect(),
            );
        }
        new_pat
    }
}

/// Evaluator for symmetric and multi-square position patterns
pub struct TransformMatcher;

impl TransformMatcher {
    /// Check if a position matches any of the transformed variants under the given symmetry
    pub fn matches_with_symmetry(
        pattern: &PositionPattern,
        pos: &Chess,
        symmetry: BoardSymmetry,
    ) -> bool {
        let concrete_symmetries = symmetry.expand();
        for sym in concrete_symmetries {
            let transformed_pattern = sym.transform_position_pattern(pattern);
            if super::pattern::PositionMatcher::matches(&transformed_pattern, pos) {
                return true;
            }
        }
        false
    }
}
