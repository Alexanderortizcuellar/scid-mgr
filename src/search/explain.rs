use shakmaty::{Bitboard, Color, Role};

use super::annotation::AnnotationPredicate;
use super::query::{
    ComparisonOp, CqlLinePattern, CqlPathConstituent, CqlPathPattern, HeaderPredicate,
    LineDirection, MaterialPredicate, MovePattern, PathPattern, PathStep, PawnPredicate,
    PieceMatcher, PositionPattern, PowerPredicate, SearchQuery, SetPredicate, SquareContent,
    SquareOrPiece, SquareSetExpr, TacticalPredicate, VariableDomain,
};
use super::transform::BoardSymmetry;

/// Formats AST search query nodes and expressions into canonical, clean CQL DSL syntax.
pub trait ToDsl {
    fn to_dsl(&self) -> String;
}

impl ToDsl for ComparisonOp {
    fn to_dsl(&self) -> String {
        match self {
            ComparisonOp::Equal => "==".to_string(),
            ComparisonOp::NotEqual => "!=".to_string(),
            ComparisonOp::GreaterThan => ">".to_string(),
            ComparisonOp::GreaterThanOrEqual => ">=".to_string(),
            ComparisonOp::LessThan => "<".to_string(),
            ComparisonOp::LessThanOrEqual => "<=".to_string(),
            ComparisonOp::Contains => "=~".to_string(),
            ComparisonOp::StartsWith => "^=".to_string(),
            ComparisonOp::EndsWith => "$=".to_string(),
            ComparisonOp::Regex => "~".to_string(),
        }
    }
}

impl ToDsl for PieceMatcher {
    fn to_dsl(&self) -> String {
        match (self.color, self.role) {
            (None, None) => "_".to_string(),
            (Some(Color::White), None) => "A".to_string(),
            (Some(Color::Black), None) => "a".to_string(),
            (None, Some(role)) => match role {
                Role::Pawn => "[Pp]".to_string(),
                Role::Knight => "[Nn]".to_string(),
                Role::Bishop => "[Bb]".to_string(),
                Role::Rook => "[Rr]".to_string(),
                Role::Queen => "[Qq]".to_string(),
                Role::King => "[Kk]".to_string(),
            },
            (Some(Color::White), Some(role)) => match role {
                Role::Pawn => "P".to_string(),
                Role::Knight => "N".to_string(),
                Role::Bishop => "B".to_string(),
                Role::Rook => "R".to_string(),
                Role::Queen => "Q".to_string(),
                Role::King => "K".to_string(),
            },
            (Some(Color::Black), Some(role)) => match role {
                Role::Pawn => "p".to_string(),
                Role::Knight => "n".to_string(),
                Role::Bishop => "b".to_string(),
                Role::Rook => "r".to_string(),
                Role::Queen => "q".to_string(),
                Role::King => "k".to_string(),
            },
        }
    }
}

impl ToDsl for SquareOrPiece {
    fn to_dsl(&self) -> String {
        match self {
            SquareOrPiece::Square(sq) => format!("{sq}"),
            SquareOrPiece::Piece(pm) => pm.to_dsl(),
            SquareOrPiece::Variable(v) => format!("${v}"),
            SquareOrPiece::Empty => ".".to_string(),
        }
    }
}

impl ToDsl for SquareContent {
    fn to_dsl(&self) -> String {
        match self {
            SquareContent::Empty => ".".to_string(),
            SquareContent::Occupied => "?".to_string(),
            SquareContent::Piece(p) => {
                let ch = p.role.char();
                if p.color == Color::White {
                    ch.to_ascii_uppercase().to_string()
                } else {
                    ch.to_ascii_lowercase().to_string()
                }
            }
            SquareContent::Role(r) => match r {
                Role::Pawn => "[Pp]".to_string(),
                Role::Knight => "[Nn]".to_string(),
                Role::Bishop => "[Bb]".to_string(),
                Role::Rook => "[Rr]".to_string(),
                Role::Queen => "[Qq]".to_string(),
                Role::King => "[Kk]".to_string(),
            },
            SquareContent::Color(c) => match c {
                Color::White => "A".to_string(),
                Color::Black => "a".to_string(),
            },
            SquareContent::AnyOf(pieces) => {
                let chars: String = pieces
                    .iter()
                    .map(|p| {
                        let ch = p.role.char();
                        if p.color == Color::White {
                            ch.to_ascii_uppercase()
                        } else {
                            ch.to_ascii_lowercase()
                        }
                    })
                    .collect();
                format!("[{chars}]")
            }
            SquareContent::NoneOf(pieces) => {
                let chars: String = pieces
                    .iter()
                    .map(|p| {
                        let ch = p.role.char();
                        if p.color == Color::White {
                            ch.to_ascii_uppercase()
                        } else {
                            ch.to_ascii_lowercase()
                        }
                    })
                    .collect();
                format!("[^{chars}]")
            }
        }
    }
}

impl ToDsl for SquareSetExpr {
    fn to_dsl(&self) -> String {
        match self {
            SquareSetExpr::Piece(content) => content.to_dsl(),
            SquareSetExpr::Squares(bb) => {
                if *bb == Bitboard::LIGHT_SQUARES {
                    "light".to_string()
                } else if *bb == Bitboard::DARK_SQUARES {
                    "dark".to_string()
                } else {
                    let sqs: Vec<String> = bb.into_iter().map(|s| format!("{s}")).collect();
                    if sqs.len() == 1 {
                        sqs[0].clone()
                    } else {
                        format!("[{}]", sqs.join(", "))
                    }
                }
            }
            SquareSetExpr::Variable(v) => format!("${v}"),
            SquareSetExpr::Intersection(left, right) => {
                format!("({} & {})", left.to_dsl(), right.to_dsl())
            }
            SquareSetExpr::Union(left, right) => {
                format!("({} | {})", left.to_dsl(), right.to_dsl())
            }
            SquareSetExpr::Difference(left, right) => {
                format!("({} \\ {})", left.to_dsl(), right.to_dsl())
            }
            SquareSetExpr::Complement(inner) => {
                format!("~({})", inner.to_dsl())
            }
            SquareSetExpr::Attacks { attacker, target } => {
                format!("attacks({}, {})", attacker.to_dsl(), target.to_dsl())
            }
            SquareSetExpr::Attackers { attacker, target } => {
                format!("attackers({}, {})", attacker.to_dsl(), target.to_dsl())
            }
            SquareSetExpr::Ray { direction, origin } => {
                format!("ray({:?}, {})", direction, origin.to_dsl())
            }
            SquareSetExpr::Between { from, to } => {
                format!("between({}, {})", from.to_dsl(), to.to_dsl())
            }
            SquareSetExpr::Shift {
                direction,
                min_dist,
                max_dist,
                expr,
            } => {
                let dir_str = format!("{:?}", direction).to_lowercase();
                if min_dist == max_dist {
                    format!("({} {} {})", dir_str, min_dist, expr.to_dsl())
                } else {
                    format!("({} {}..{} {})", dir_str, min_dist, max_dist, expr.to_dsl())
                }
            }
        }
    }
}

impl ToDsl for SetPredicate {
    fn to_dsl(&self) -> String {
        match self {
            SetPredicate::NonEmpty(expr) => expr.to_dsl(),
            SetPredicate::CountComparison { expr, op, count } => {
                format!("{} {} {count}", expr.to_dsl(), op.to_dsl())
            }
            SetPredicate::SetComparison { left, op, right } => {
                format!("{} {} {}", left.to_dsl(), op.to_dsl(), right.to_dsl())
            }
        }
    }
}

impl ToDsl for HeaderPredicate {
    fn to_dsl(&self) -> String {
        match self {
            HeaderPredicate::Player { name, op, .. } => {
                if *op == ComparisonOp::Equal || *op == ComparisonOp::Contains {
                    format!("player \"{name}\"")
                } else {
                    format!("player {} \"{name}\"", op.to_dsl())
                }
            }
            HeaderPredicate::White { name, op, .. } => {
                if *op == ComparisonOp::Equal || *op == ComparisonOp::Contains {
                    format!("white \"{name}\"")
                } else {
                    format!("white {} \"{name}\"", op.to_dsl())
                }
            }
            HeaderPredicate::Black { name, op, .. } => {
                if *op == ComparisonOp::Equal || *op == ComparisonOp::Contains {
                    format!("black \"{name}\"")
                } else {
                    format!("black {} \"{name}\"", op.to_dsl())
                }
            }
            HeaderPredicate::WhiteElo { op, value } => format!("white_elo {} {value}", op.to_dsl()),
            HeaderPredicate::BlackElo { op, value } => format!("black_elo {} {value}", op.to_dsl()),
            HeaderPredicate::AnyElo { op, value } => format!("elo {} {value}", op.to_dsl()),
            HeaderPredicate::AvgElo { op, value } => format!("avg_elo {} {value}", op.to_dsl()),
            HeaderPredicate::EloDiff {
                op,
                value,
                absolute,
            } => {
                if *absolute {
                    format!("elo_diff {} {value}", op.to_dsl())
                } else {
                    format!("raw_elo_diff {} {value}", op.to_dsl())
                }
            }
            HeaderPredicate::Result { expected } => format!("result \"{expected}\""),
            HeaderPredicate::Eco { code, op } => {
                if *op == ComparisonOp::Equal || *op == ComparisonOp::StartsWith {
                    format!("eco \"{code}\"")
                } else {
                    format!("eco {} \"{code}\"", op.to_dsl())
                }
            }
            HeaderPredicate::Date { op, value } => format!("date {} \"{value}\"", op.to_dsl()),
            HeaderPredicate::Event { name, op, .. } => {
                if *op == ComparisonOp::Equal || *op == ComparisonOp::Contains {
                    format!("event \"{name}\"")
                } else {
                    format!("event {} \"{name}\"", op.to_dsl())
                }
            }
            HeaderPredicate::Site { name, op, .. } => {
                if *op == ComparisonOp::Equal || *op == ComparisonOp::Contains {
                    format!("site \"{name}\"")
                } else {
                    format!("site {} \"{name}\"", op.to_dsl())
                }
            }
            HeaderPredicate::Round { value } => format!("round \"{value}\""),
            HeaderPredicate::Custom { key, op, value } => {
                format!("header \"{key}\" {} \"{value}\"", op.to_dsl())
            }
            HeaderPredicate::Tag {
                name, op, value, ..
            } => {
                format!("tag \"{name}\" {} \"{value}\"", op.to_dsl())
            }
        }
    }
}

impl ToDsl for PositionPattern {
    fn to_dsl(&self) -> String {
        match self {
            PositionPattern::ExactFen(fen) => format!("fen \"{fen}\""),
            PositionPattern::PiecePlacement(placement) => {
                format!("piece_placement \"{placement}\"")
            }
            PositionPattern::ZobristHash(hash) => format!("zobrist {hash:#x}"),
            PositionPattern::Squares(sqs) => {
                let mut parts = Vec::new();
                let mut sorted_sqs: Vec<_> = sqs.iter().collect();
                sorted_sqs.sort_by_key(|(s, _)| **s);
                for (sq, content) in sorted_sqs {
                    parts.push(format!("{} on {sq}", content.to_dsl()));
                }
                parts.join(" ")
            }
            PositionPattern::PieceCount {
                content,
                squares,
                op,
                count,
            } => {
                if let Some(sqs) = squares {
                    let sq_strs: Vec<String> = sqs.iter().map(|s| format!("{s}")).collect();
                    format!(
                        "count {} on [{}] {} {count}",
                        content.to_dsl(),
                        sq_strs.join(" "),
                        op.to_dsl()
                    )
                } else {
                    format!("{} {} {count}", content.to_dsl(), op.to_dsl())
                }
            }
            PositionPattern::Turn(Color::White) => "wtm".to_string(),
            PositionPattern::Turn(Color::Black) => "btm".to_string(),
            PositionPattern::Castling {
                color,
                kingside,
                queenside,
            } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                match (kingside, queenside) {
                    (Some(true), None) => format!("{col}_castling O-O"),
                    (None, Some(true)) => format!("{col}_castling O-O-O"),
                    (Some(true), Some(true)) => format!("{col}_castling all"),
                    (Some(false), Some(false)) => format!("{col}_castling none"),
                    _ => format!("{col}_castling"),
                }
            }
            PositionPattern::BoardState {
                is_check,
                is_checkmate,
                is_stalemate,
            } => {
                let mut parts = Vec::new();
                if let Some(c) = is_check {
                    parts.push(if *c { "check" } else { "not check" });
                }
                if let Some(m) = is_checkmate {
                    parts.push(if *m { "checkmate" } else { "not checkmate" });
                }
                if let Some(s) = is_stalemate {
                    parts.push(if *s { "stalemate" } else { "not stalemate" });
                }
                parts.join(" and ")
            }
            PositionPattern::Attack { from, to } => format!("attack {from} -> {to}"),
            PositionPattern::IsAttacked { square, by_color } => {
                let col = if *by_color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("attacked {square} by {col}")
            }
            PositionPattern::MultiSquare { content, squares } => {
                let sq_strs: Vec<String> = squares.iter().map(|s| format!("{s}")).collect();
                format!("{} on [{}]", content.to_dsl(), sq_strs.join(" "))
            }
            PositionPattern::Symmetric { pattern, symmetry } => {
                let sym_str = match symmetry {
                    BoardSymmetry::HorizontalMirror => "flip:horizontal",
                    BoardSymmetry::VerticalMirror => "flip:vertical",
                    BoardSymmetry::MainDiagonal => "flip:maindiagonal",
                    BoardSymmetry::AntiDiagonal => "flip:antidiagonal",
                    BoardSymmetry::Rotate180 => "flip:rotate180",
                    BoardSymmetry::Rotate90 => "rotate90",
                    BoardSymmetry::Rotate270 => "rotate270",
                    BoardSymmetry::AllRotations => "rotate90",
                    BoardSymmetry::ColorInvert => "flipcolor",
                    BoardSymmetry::ColorInvertHorizontal => "flipcolor:horizontal",
                    BoardSymmetry::AnySpatialSymmetry => "flip:spatial",
                    BoardSymmetry::AnyTotalSymmetry => "flip:all",
                    BoardSymmetry::Identity => "flip:none",
                };
                format!("{sym_str} {{ {} }}", pattern.to_dsl())
            }
            PositionPattern::Ply { op, value } => format!("ply {} {value}", op.to_dsl()),
            PositionPattern::MoveNumber { op, value } => {
                format!("move_number {} {value}", op.to_dsl())
            }
            PositionPattern::VariableSquareFilter { var_name, squares } => {
                let sq_strs: Vec<String> = squares.iter().map(|s| format!("{s}")).collect();
                format!("${var_name} on [{}]", sq_strs.join(" "))
            }
        }
    }
}

impl ToDsl for MovePattern {
    fn to_dsl(&self) -> String {
        let mut parts = Vec::new();
        if self.is_legal {
            parts.push("legal".to_string());
        } else {
            parts.push("move".to_string());
        }

        if self.is_previous {
            parts.push("previous".to_string());
        }

        if let Some(color) = self.color {
            parts.push(
                if color == Color::White {
                    "white"
                } else {
                    "black"
                }
                .to_string(),
            );
        }
        if let Some(role) = self.role {
            parts.push(
                match role {
                    Role::Pawn => "P",
                    Role::Knight => "N",
                    Role::Bishop => "B",
                    Role::Rook => "R",
                    Role::Queen => "Q",
                    Role::King => "K",
                }
                .to_string(),
            );
        }

        if let Some(san) = &self.san {
            parts.push(format!("\"{san}\""));
        } else if let Some(uci) = &self.uci {
            parts.push(uci.clone());
        } else {
            if let Some(from) = self.from {
                parts.push(format!("from {from}"));
            } else if let Some(from_sqs) = &self.from_squares {
                let sqs: Vec<String> = from_sqs.iter().map(|s| format!("{s}")).collect();
                parts.push(format!("from [{}]", sqs.join(" ")));
            } else if let Some(from_pcs) = &self.from_pieces {
                let pcs: Vec<String> = from_pcs.iter().map(|p| p.to_dsl()).collect();
                parts.push(format!("from [{}]", pcs.join(" ")));
            }

            if let Some(to) = self.to {
                parts.push(format!("to {to}"));
            } else if let Some(to_sqs) = &self.to_squares {
                let sqs: Vec<String> = to_sqs.iter().map(|s| format!("{s}")).collect();
                parts.push(format!("to [{}]", sqs.join(" ")));
            } else if let Some(to_pcs) = &self.to_pieces {
                let pcs: Vec<String> = to_pcs.iter().map(|p| p.to_dsl()).collect();
                parts.push(format!("to [{}]", pcs.join(" ")));
            }
        }

        if let Some((dir, min, max_opt)) = self.direction {
            let dir_name = match dir {
                crate::search::query::Direction::Up => "up",
                crate::search::query::Direction::Down => "down",
                crate::search::query::Direction::Left => "left",
                crate::search::query::Direction::Right => "right",
                crate::search::query::Direction::NorthEast => "northeast",
                crate::search::query::Direction::NorthWest => "northwest",
                crate::search::query::Direction::SouthEast => "southeast",
                crate::search::query::Direction::SouthWest => "southwest",
                crate::search::query::Direction::Diagonal => "diagonal",
                crate::search::query::Direction::Orthogonal => "orthogonal",
                crate::search::query::Direction::Vertical => "vertical",
                crate::search::query::Direction::Horizontal => "horizontal",
                crate::search::query::Direction::AnyDirection => "anydirection",
            };
            if let Some(max) = max_opt {
                if max == min {
                    parts.push(format!("{dir_name} {min}"));
                } else {
                    parts.push(format!("{dir_name} {min} {max}"));
                }
            } else {
                parts.push(dir_name.to_string());
            }
        }

        if let Some(is_castle) = self.is_castle {
            if is_castle {
                parts.push("castle".to_string());
            }
        }
        if let Some(is_ep) = self.is_en_passant {
            if is_ep {
                parts.push("en_passant".to_string());
            }
        }
        if let Some(is_cap) = self.is_capture {
            if is_cap {
                if let Some(ref cap_pcs) = self.captured_pieces {
                    if cap_pcs.len() == 1 {
                        parts.push(format!("capture {}", cap_pcs[0].to_dsl()));
                    } else {
                        let pcs: Vec<String> = cap_pcs.iter().map(|p| p.to_dsl()).collect();
                        parts.push(format!("capture [{}]", pcs.join(" ")));
                    }
                } else {
                    parts.push("capture".to_string());
                }
            } else {
                parts.push("not capture".to_string());
            }
        }
        if let Some(promo) = self.promotion {
            parts.push(format!("promote {}", promo.char().to_ascii_uppercase()));
        } else if let Some(promos) = &self.promotions {
            let chs: String = promos
                .iter()
                .map(|r| r.char().to_ascii_uppercase())
                .collect();
            parts.push(format!("promote [{chs}]"));
        }
        if let Some(is_chk) = self.is_check {
            if is_chk {
                parts.push("check".to_string());
            }
        }
        if let Some(is_mate) = self.is_checkmate {
            if is_mate {
                parts.push("mate".to_string());
            }
        }
        if let Some((op, cnt)) = self.count_predicate {
            parts.push(format!("count {} {cnt}", op.to_dsl()));
        }

        parts.join(" ")
    }
}

impl ToDsl for PathPattern {
    fn to_dsl(&self) -> String {
        let mut steps_dsl = Vec::new();
        if !self.steps.is_empty() {
            for step in &self.steps {
                match step {
                    PathStep::Move {
                        pattern,
                        repeat_min,
                        repeat_max,
                    } => {
                        let m_str = pattern.to_dsl();
                        match (repeat_min, repeat_max) {
                            (1, Some(1)) => steps_dsl.push(m_str),
                            (min, Some(max)) if min == max => {
                                steps_dsl.push(format!("{m_str}{{{min}}}"));
                            }
                            (min, Some(max)) => {
                                steps_dsl.push(format!("{m_str}{{{min},{max}}}"));
                            }
                            (min, None) => {
                                steps_dsl.push(format!("{m_str}{{{min},}}"));
                            }
                        }
                    }
                    PathStep::Gap { min, max } => match (min, max) {
                        (0, None) => steps_dsl.push("...".to_string()),
                        (1, None) => steps_dsl.push("--+".to_string()),
                        (0, Some(0)) => {}
                        (min_val, Some(max_val)) if min_val == max_val => {
                            steps_dsl.push(format!("--{{{min_val}}}"));
                        }
                        (min_val, Some(max_val)) => {
                            steps_dsl.push(format!("--{{{min_val},{max_val}}}"));
                        }
                        (min_val, None) => {
                            steps_dsl.push(format!("--{{{min_val},}}"));
                        }
                    },
                }
            }
        } else {
            for m in &self.moves {
                steps_dsl.push(m.to_dsl());
            }
        }

        let body = steps_dsl.join(" ");
        let color_qualifier = match self.single_color {
            Some(Some(Color::White)) => " white",
            Some(Some(Color::Black)) => " black",
            Some(None) => " singlecolor",
            None => "",
        };

        if let Some(range) = &self.start_ply_range {
            format!(
                "path{color_qualifier} from ply {} to {} {{ {body} }}",
                range.start, range.end
            )
        } else {
            format!("path{color_qualifier} {{ {body} }}")
        }
    }
}

impl ToDsl for CqlPathConstituent {
    fn to_dsl(&self) -> String {
        match self {
            CqlPathConstituent::Move(m) => m.to_dsl(),
            CqlPathConstituent::Filter(q) => format!("{{ {} }}", q.to_dsl()),
            CqlPathConstituent::Chain(subs) => {
                let inner = subs
                    .iter()
                    .map(|s| s.to_dsl())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("( {inner} )")
            }
            CqlPathConstituent::Repetition {
                constituent,
                min,
                max,
            } => {
                let inner = constituent.to_dsl();
                match (min, max) {
                    (0, None) => format!("{inner}*"),
                    (1, None) => format!("{inner}+"),
                    (0, Some(1)) => format!("{inner}?"),
                    (min_val, Some(max_val)) if min_val == max_val => {
                        format!("{inner}{{{min_val}}}")
                    }
                    (min_val, Some(max_val)) => format!("{inner}{{{min_val},{max_val}}}"),
                    (min_val, None) => format!("{inner}{{{min_val},}}"),
                }
            }
        }
    }
}

impl ToDsl for CqlPathPattern {
    fn to_dsl(&self) -> String {
        let body = self
            .constituents
            .iter()
            .map(|c| c.to_dsl())
            .collect::<Vec<_>>()
            .join(" ");

        let color_qualifier = match self.single_color {
            Some(Some(Color::White)) => " white",
            Some(Some(Color::Black)) => " black",
            Some(None) => " singlecolor",
            None => "",
        };

        if let Some(range) = &self.start_ply_range {
            format!(
                "cql_path{color_qualifier} from ply {} to {} {{ {body} }}",
                range.start, range.end
            )
        } else {
            format!("cql_path{color_qualifier} {{ {body} }}")
        }
    }
}

impl ToDsl for CqlLinePattern {
    fn to_dsl(&self) -> String {
        let arrow = match self.direction {
            LineDirection::Forward => "-->",
            LineDirection::Backward => "<--",
        };

        let mut parts = vec!["line".to_string()];

        if let (Some(min), Some(max)) = (self.min_length, self.max_length) {
            if min == max {
                parts.push(format!("{min}"));
            } else {
                parts.push(format!("{min} {max}"));
            }
        } else if let Some(min) = self.min_length {
            parts.push(format!("{min}"));
        }

        if self.first_match {
            parts.push("firstmatch".to_string());
        }
        if self.last_position {
            parts.push("lastposition".to_string());
        }
        if self.nest_ban {
            parts.push("nestban".to_string());
        }

        match self.single_color {
            Some(Some(Color::White)) => parts.push("white".to_string()),
            Some(Some(Color::Black)) => parts.push("black".to_string()),
            Some(None) => parts.push("singlecolor".to_string()),
            None => {}
        }

        let body = self
            .constituents
            .iter()
            .map(|c| c.to_dsl())
            .collect::<Vec<_>>()
            .join(&format!(" {arrow} "));

        if !body.is_empty() {
            parts.push(arrow.to_string());
            parts.push(body);
        }

        parts.join(" ")
    }
}

impl ToDsl for TacticalPredicate {
    fn to_dsl(&self) -> String {
        match self {
            TacticalPredicate::Pin {
                pinners,
                pinneds,
                targets,
            } => {
                let mut parts = vec!["pin".to_string()];
                if !pinners.is_empty() {
                    let s: Vec<String> = pinners.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("pinner [{}]", s.join(" ")));
                }
                if !pinneds.is_empty() {
                    let s: Vec<String> = pinneds.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("pinned [{}]", s.join(" ")));
                }
                if !targets.is_empty() {
                    let s: Vec<String> = targets.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("target [{}]", s.join(" ")));
                }
                parts.join(" ")
            }
            TacticalPredicate::Fork {
                attackers,
                target_slots,
                targets_pool,
                min_targets,
            } => {
                let mut parts = vec!["fork".to_string()];
                if !attackers.is_empty() {
                    let s: Vec<String> = attackers.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("attacker [{}]", s.join(" ")));
                }
                for slot in target_slots {
                    let s: Vec<String> = slot.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("target [{}]", s.join(" ")));
                }
                if !targets_pool.is_empty() {
                    let s: Vec<String> = targets_pool.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("targets [{}]", s.join(" ")));
                }
                if *min_targets > 2 {
                    parts.push(format!("min_targets {min_targets}"));
                }
                parts.join(" ")
            }
            TacticalPredicate::DiscoveredAttack { color, is_check } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                if *is_check {
                    format!("discovered_check {col}")
                } else {
                    format!("discovered_attack {col}")
                }
            }
            TacticalPredicate::Skewer {
                attackers,
                fronts,
                rears,
            } => {
                let mut parts = vec!["skewer".to_string()];
                if !attackers.is_empty() {
                    let s: Vec<String> = attackers.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("attacker [{}]", s.join(" ")));
                }
                if !fronts.is_empty() {
                    let s: Vec<String> = fronts.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("front [{}]", s.join(" ")));
                }
                if !rears.is_empty() {
                    let s: Vec<String> = rears.iter().map(|p| p.to_dsl()).collect();
                    parts.push(format!("rear [{}]", s.join(" ")));
                }
                parts.join(" ")
            }
            TacticalPredicate::TrappedPiece { piece } => format!("trapped {}", piece.to_dsl()),
            TacticalPredicate::Outpost { piece, square } => {
                if let Some(sq) = square {
                    format!("outpost {} on {sq}", piece.to_dsl())
                } else {
                    format!("outpost {}", piece.to_dsl())
                }
            }
            TacticalPredicate::RookOnSeventh { color } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("rook_on_seventh {col}")
            }
            TacticalPredicate::OpenFile {
                file,
                semi_open_for,
            } => {
                let f_str = file
                    .map(|f| format!("{f}"))
                    .unwrap_or_else(|| "_".to_string());
                if let Some(col) = semi_open_for {
                    let c_str = if *col == Color::White {
                        "white"
                    } else {
                        "black"
                    };
                    format!("semi_open_file {f_str} for {c_str}")
                } else {
                    format!("open_file {f_str}")
                }
            }
            TacticalPredicate::Distance {
                sq1,
                sq2,
                op,
                distance,
            } => {
                format!(
                    "distance({}, {}) {} {distance}",
                    sq1.to_dsl(),
                    sq2.to_dsl(),
                    op.to_dsl()
                )
            }
            TacticalPredicate::Attacks { attacker, target } => {
                format!("attacks({}, {})", attacker.to_dsl(), target.to_dsl())
            }
        }
    }
}

impl ToDsl for MaterialPredicate {
    fn to_dsl(&self) -> String {
        let mut parts = Vec::new();
        if let Some(c) = self.white_pawns {
            parts.push(format!("P == {c}"));
        }
        if let Some(c) = self.white_knights {
            parts.push(format!("N == {c}"));
        }
        if let Some(c) = self.white_bishops {
            parts.push(format!("B == {c}"));
        }
        if let Some(c) = self.white_rooks {
            parts.push(format!("R == {c}"));
        }
        if let Some(c) = self.white_queens {
            parts.push(format!("Q == {c}"));
        }
        if let Some(c) = self.black_pawns {
            parts.push(format!("p == {c}"));
        }
        if let Some(c) = self.black_knights {
            parts.push(format!("n == {c}"));
        }
        if let Some(c) = self.black_bishops {
            parts.push(format!("b == {c}"));
        }
        if let Some(c) = self.black_rooks {
            parts.push(format!("r == {c}"));
        }
        if let Some(c) = self.black_queens {
            parts.push(format!("q == {c}"));
        }
        if let Some((op, diff)) = self.material_difference {
            parts.push(format!("material_diff {} {diff}", op.to_dsl()));
        }
        if let Some(true) = self.opposite_bishops {
            parts.push("opposite_bishops".to_string());
        }
        if let Some(true) = self.same_colored_bishops {
            parts.push("same_colored_bishops".to_string());
        }
        if parts.is_empty() {
            "material".to_string()
        } else {
            parts.join(" and ")
        }
    }
}

impl ToDsl for PawnPredicate {
    fn to_dsl(&self) -> String {
        match self {
            PawnPredicate::PassedPawns { color, op, count } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("passed_pawns {col} {} {count}", op.to_dsl())
            }
            PawnPredicate::IsolatedPawns { color, op, count } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("isolated_pawns {col} {} {count}", op.to_dsl())
            }
            PawnPredicate::DoubledPawns { color, op, count } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("doubled_pawns {col} {} {count}", op.to_dsl())
            }
            PawnPredicate::BackwardPawns { color, op, count } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("backward_pawns {col} {} {count}", op.to_dsl())
            }
            PawnPredicate::PawnIslands { color, op, count } => {
                let col = if *color == Color::White {
                    "white"
                } else {
                    "black"
                };
                format!("pawn_islands {col} {} {count}", op.to_dsl())
            }
        }
    }
}

impl ToDsl for PowerPredicate {
    fn to_dsl(&self) -> String {
        match self {
            PowerPredicate::WhitePower { op, value } => {
                format!("white_power {} {value}", op.to_dsl())
            }
            PowerPredicate::BlackPower { op, value } => {
                format!("black_power {} {value}", op.to_dsl())
            }
            PowerPredicate::TotalPower { op, value } => {
                format!("total_power {} {value}", op.to_dsl())
            }
            PowerPredicate::WhiteVsBlackPower { op } => {
                format!("white_power {} black_power", op.to_dsl())
            }
            PowerPredicate::PowerDifference {
                op,
                value,
                absolute,
            } => {
                if *absolute {
                    format!("power_diff {} {value}", op.to_dsl())
                } else {
                    format!("raw_power_diff {} {value}", op.to_dsl())
                }
            }
        }
    }
}

impl ToDsl for AnnotationPredicate {
    fn to_dsl(&self) -> String {
        "annotation".to_string()
    }
}

impl ToDsl for SearchQuery {
    fn to_dsl(&self) -> String {
        match self {
            SearchQuery::And(subs) => {
                if subs.is_empty() {
                    return "".to_string();
                }
                if subs.len() == 1 {
                    return subs[0].to_dsl();
                }
                subs.iter()
                    .map(|s| {
                        let inner = s.to_dsl();
                        if matches!(s, SearchQuery::Or(_)) {
                            format!("({inner})")
                        } else {
                            inner
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" and ")
            }
            SearchQuery::Or(subs) => {
                if subs.is_empty() {
                    return "".to_string();
                }
                if subs.len() == 1 {
                    return subs[0].to_dsl();
                }
                subs.iter()
                    .map(|s| s.to_dsl())
                    .collect::<Vec<_>>()
                    .join(" or ")
            }
            SearchQuery::Not(sub) => {
                let inner = sub.to_dsl();
                if matches!(**sub, SearchQuery::And(_) | SearchQuery::Or(_)) {
                    format!("not ({inner})")
                } else {
                    format!("not {inner}")
                }
            }
            SearchQuery::Parent(sub) => {
                format!("parent {{ {} }}", sub.to_dsl())
            }
            SearchQuery::Child(sub) => {
                format!("child {{ {} }}", sub.to_dsl())
            }
            SearchQuery::Play {
                move_pattern,
                outcome_query,
            } => {
                let move_dsl = move_pattern.to_dsl();
                if move_dsl.is_empty() {
                    format!("legal leads_to {{ {} }}", outcome_query.to_dsl())
                } else {
                    format!("{move_dsl} leads_to {{ {} }}", outcome_query.to_dsl())
                }
            }
            SearchQuery::Header(h) => h.to_dsl(),
            SearchQuery::Position(p) => p.to_dsl(),
            SearchQuery::Pawn(p) => p.to_dsl(),
            SearchQuery::Tactical(t) => t.to_dsl(),
            SearchQuery::Move(m) => m.to_dsl(),
            SearchQuery::Path(p) => p.to_dsl(),
            SearchQuery::Material(m) => m.to_dsl(),
            SearchQuery::Power(p) => p.to_dsl(),
            SearchQuery::Annotation(a) => a.to_dsl(),
            SearchQuery::Symmetric { query, symmetry } => {
                let sym_str = match symmetry {
                    BoardSymmetry::HorizontalMirror => "flip:horizontal",
                    BoardSymmetry::VerticalMirror => "flip:vertical",
                    BoardSymmetry::MainDiagonal => "flip:maindiagonal",
                    BoardSymmetry::AntiDiagonal => "flip:antidiagonal",
                    BoardSymmetry::Rotate180 => "flip:rotate180",
                    BoardSymmetry::Rotate90 => "rotate90",
                    BoardSymmetry::Rotate270 => "rotate270",
                    BoardSymmetry::AllRotations => "rotate90",
                    BoardSymmetry::ColorInvert => "flipcolor",
                    BoardSymmetry::ColorInvertHorizontal => "flipcolor:horizontal",
                    BoardSymmetry::AnySpatialSymmetry => "flip:spatial",
                    BoardSymmetry::AnyTotalSymmetry => "flip:all",
                    BoardSymmetry::Identity => "flip:none",
                };
                format!("{sym_str} {{ {} }}", query.to_dsl())
            }
            SearchQuery::Shift { mode, query } => {
                let mode_str = match mode {
                    crate::search::transform::ShiftMode::Horizontal => "shifthorizontal",
                    crate::search::transform::ShiftMode::Vertical => "shiftvertical",
                    crate::search::transform::ShiftMode::All => "shift",
                };
                format!("{mode_str} {{ {} }}", query.to_dsl())
            }
            SearchQuery::PlyRange { range, query } => {
                format!(
                    "ply in {}..{} {{ {} }}",
                    range.start,
                    range.end,
                    query.to_dsl()
                )
            }
            SearchQuery::Occurrences { min, max, query } => {
                if let Some(m) = max {
                    format!("occurrences {min}..{m} {{ {} }}", query.to_dsl())
                } else {
                    format!("occurrences >={min} {{ {} }}", query.to_dsl())
                }
            }
            SearchQuery::SquareSet(s) => s.to_dsl(),
            SearchQuery::CqlPath(p) => p.to_dsl(),
            SearchQuery::CqlLine(p) => p.to_dsl(),
            SearchQuery::VariableBinding {
                var_name,
                domain,
                query,
            } => {
                let domain_str = match domain {
                    VariableDomain::Piece(pm) => pm.to_dsl(),
                    VariableDomain::SquareSet(sqs) => {
                        let s: Vec<String> = sqs.iter().map(|sq| format!("{sq}")).collect();
                        format!("[{}]", s.join(" "))
                    }
                    VariableDomain::AnyPiece => "_".to_string(),
                };
                format!("piece ${var_name} in {domain_str} {{ {} }}", query.to_dsl())
            }
            SearchQuery::Initial(q) => format!("initial {{ {} }}", q.to_dsl()),
            SearchQuery::Terminal(q) => format!("terminal {{ {} }}", q.to_dsl()),
        }
    }
}

/// Detailed explanation and transformation analysis of a CQL search query
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryBranch {
    pub symmetry_name: String,
    pub dsl: String,
    pub transformed_fens: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryExplanation {
    pub original_query: String,
    pub canonical_dsl: String,
    pub is_header_only: bool,
    pub has_symmetries: bool,
    pub branches: Vec<QueryBranch>,
}

fn collect_fens(query: &SearchQuery, out: &mut Vec<String>) {
    match query {
        SearchQuery::Position(PositionPattern::ExactFen(fen)) => {
            out.push(fen.clone());
        }
        SearchQuery::Position(PositionPattern::PiecePlacement(placement)) => {
            out.push(placement.clone());
        }
        SearchQuery::Position(PositionPattern::Symmetric { pattern, .. }) => {
            if let PositionPattern::ExactFen(fen) = &**pattern {
                out.push(fen.clone());
            } else if let PositionPattern::PiecePlacement(pl) = &**pattern {
                out.push(pl.clone());
            }
        }
        SearchQuery::And(subs) | SearchQuery::Or(subs) => {
            for sub in subs {
                collect_fens(sub, out);
            }
        }
        SearchQuery::Not(sub)
        | SearchQuery::PlyRange { query: sub, .. }
        | SearchQuery::Occurrences { query: sub, .. }
        | SearchQuery::VariableBinding { query: sub, .. }
        | SearchQuery::Symmetric { query: sub, .. }
        | SearchQuery::Shift { query: sub, .. }
        | SearchQuery::Initial(sub)
        | SearchQuery::Terminal(sub) => {
            collect_fens(sub, out);
        }
        _ => {}
    }
}

fn contains_symmetry(query: &SearchQuery) -> bool {
    match query {
        SearchQuery::Symmetric { .. } => true,
        SearchQuery::Position(PositionPattern::Symmetric { .. }) => true,
        SearchQuery::And(subs) | SearchQuery::Or(subs) => subs.iter().any(contains_symmetry),
        SearchQuery::Not(sub)
        | SearchQuery::PlyRange { query: sub, .. }
        | SearchQuery::Occurrences { query: sub, .. }
        | SearchQuery::VariableBinding { query: sub, .. }
        | SearchQuery::Shift { query: sub, .. }
        | SearchQuery::Initial(sub)
        | SearchQuery::Terminal(sub) => contains_symmetry(sub),
        _ => false,
    }
}

fn symmetry_label(sym: BoardSymmetry) -> &'static str {
    match sym {
        BoardSymmetry::Identity => "Identity (original)",
        BoardSymmetry::ColorInvert => "Color Inverted (flipcolor)",
        BoardSymmetry::HorizontalMirror => "Horizontal Mirror (flip:horizontal)",
        BoardSymmetry::VerticalMirror => "Vertical Mirror (flip:vertical)",
        BoardSymmetry::MainDiagonal => "Main Diagonal Reflection (flip:maindiagonal)",
        BoardSymmetry::AntiDiagonal => "Anti-Diagonal Reflection (flip:antidiagonal)",
        BoardSymmetry::Rotate180 => "180° Board Rotation (flip:rotate180)",
        BoardSymmetry::Rotate90 => "90° Board Rotation (rotate90)",
        BoardSymmetry::Rotate270 => "270° Board Rotation (rotate270)",
        BoardSymmetry::AllRotations => "All Rotations (0°, 90°, 180°, 270°)",
        BoardSymmetry::ColorInvertHorizontal => "Color Invert + Horizontal Mirror",
        BoardSymmetry::AnySpatialSymmetry => "Spatial Symmetry",
        BoardSymmetry::AnyTotalSymmetry => "Total Symmetry",
    }
}

pub fn explain_query(original_input: &str, query: &SearchQuery) -> QueryExplanation {
    let canonical_dsl = query.to_dsl();
    let is_header_only = query.is_header_only();
    let has_symmetries = contains_symmetry(query);

    let mut branches = Vec::new();

    match query {
        SearchQuery::Symmetric {
            query: inner,
            symmetry,
        } => {
            let expanded_syms = symmetry.expand();
            for sym in expanded_syms {
                let transformed = sym.transform_query(inner);
                let dsl = transformed.to_dsl();
                let mut fens = Vec::new();
                collect_fens(&transformed, &mut fens);
                branches.push(QueryBranch {
                    symmetry_name: symmetry_label(sym).to_string(),
                    dsl,
                    transformed_fens: fens,
                });
            }
        }
        SearchQuery::Position(PositionPattern::Symmetric {
            pattern: inner_pat,
            symmetry,
        }) => {
            let expanded_syms = symmetry.expand();
            for sym in expanded_syms {
                let transformed_pat = sym.transform_position_pattern(inner_pat);
                let dsl = transformed_pat.to_dsl();
                let mut fens = Vec::new();
                if let PositionPattern::ExactFen(fen) = &transformed_pat {
                    fens.push(fen.clone());
                } else if let PositionPattern::PiecePlacement(pl) = &transformed_pat {
                    fens.push(pl.clone());
                }
                branches.push(QueryBranch {
                    symmetry_name: symmetry_label(sym).to_string(),
                    dsl,
                    transformed_fens: fens,
                });
            }
        }
        _ => {
            let mut fens = Vec::new();
            collect_fens(query, &mut fens);
            branches.push(QueryBranch {
                symmetry_name: "Original (Identity)".to_string(),
                dsl: canonical_dsl.clone(),
                transformed_fens: fens,
            });
        }
    }

    QueryExplanation {
        original_query: original_input.to_string(),
        canonical_dsl,
        is_header_only,
        has_symmetries,
        branches,
    }
}
