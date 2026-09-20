use super::query::{MovePattern, PathPattern};
use shakmaty::{Chess, Move, Position};

/// Executed move record containing move details and SAN representation
#[derive(Debug, Clone)]
pub struct MoveRecord {
    pub ply: usize,
    pub mv: Move,
    pub san: String,
    pub is_check: bool,
    pub nags: Vec<u8>,
    pub comment: Option<String>,
}

/// Evaluates move patterns and sequence paths against game moves
pub struct PathMatcher;

impl PathMatcher {
    /// Match a single MoveRecord against a MovePattern
    pub fn match_move(pattern: &MovePattern, pos_before: &Chess, record: &MoveRecord) -> bool {
        let mv = &record.mv;

        if let Some(ref expected_san) = pattern.san {
            let clean_expected = expected_san.trim().trim_end_matches(['+', '#', '!', '?']);
            let clean_actual = record.san.trim().trim_end_matches(['+', '#', '!', '?']);
            if clean_actual != clean_expected {
                return false;
            }
        }

        if let Some(ref expected_uci) = pattern.uci {
            let uci_str = mv.to_uci(shakmaty::CastlingMode::Standard).to_string();
            if uci_str != expected_uci.trim() {
                return false;
            }
        }

        if let Some(from_sq) = pattern.from {
            if mv.from() != Some(from_sq) {
                return false;
            }
        }

        if let Some(ref from_sqs) = pattern.from_squares {
            if let Some(from_sq) = mv.from() {
                if !from_sqs.contains(&from_sq) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref from_pcs) = pattern.from_pieces {
            if let Some(from_sq) = mv.from() {
                let piece_opt = pos_before.board().piece_at(from_sq);
                let matches = from_pcs.iter().any(|target| match target {
                    super::query::SquareContent::Piece(p) => piece_opt.as_ref() == Some(p),
                    super::query::SquareContent::Color(c) => {
                        piece_opt.as_ref().map(|p| p.color) == Some(*c)
                    }
                    super::query::SquareContent::Role(r) => {
                        piece_opt.as_ref().map(|p| p.role) == Some(*r)
                    }
                    super::query::SquareContent::Occupied => piece_opt.is_some(),
                    super::query::SquareContent::Empty => piece_opt.is_none(),
                    super::query::SquareContent::AnyOf(pieces) => piece_opt
                        .as_ref()
                        .map(|p| pieces.contains(p))
                        .unwrap_or(false),
                    super::query::SquareContent::NoneOf(pieces) => piece_opt
                        .as_ref()
                        .map(|p| !pieces.contains(p))
                        .unwrap_or(true),
                });
                if !matches {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(to_sq) = pattern.to {
            if !move_destination_matches(mv, to_sq) {
                return false;
            }
        }

        if let Some(ref to_sqs) = pattern.to_squares {
            if !move_destinations_contain(mv, to_sqs) {
                return false;
            }
        }

        if let Some(ref to_pcs) = pattern.to_pieces {
            let to_sq = if mv.is_castle() {
                match (pos_before.turn(), mv.to()) {
                    (shakmaty::Color::White, shakmaty::Square::A1) => shakmaty::Square::C1,
                    (shakmaty::Color::White, shakmaty::Square::H1) => shakmaty::Square::G1,
                    (shakmaty::Color::Black, shakmaty::Square::A8) => shakmaty::Square::C8,
                    (shakmaty::Color::Black, shakmaty::Square::H8) => shakmaty::Square::G8,
                    _ => mv.to(),
                }
            } else {
                mv.to()
            };
            let piece_opt = pos_before.board().piece_at(to_sq);
            let ep_piece_opt = if mv.is_en_passant() {
                mv.from().and_then(|from_sq| {
                    let ep_sq = shakmaty::Square::from_coords(to_sq.file(), from_sq.rank());
                    pos_before.board().piece_at(ep_sq)
                })
            } else {
                None
            };

            let matches = to_pcs.iter().any(|target| match target {
                super::query::SquareContent::Piece(p) => {
                    piece_opt.as_ref() == Some(p) || ep_piece_opt.as_ref() == Some(p)
                }
                super::query::SquareContent::Color(c) => {
                    piece_opt.as_ref().map(|p| p.color) == Some(*c)
                        || ep_piece_opt.as_ref().map(|p| p.color) == Some(*c)
                }
                super::query::SquareContent::Role(r) => {
                    piece_opt.as_ref().map(|p| p.role) == Some(*r)
                        || ep_piece_opt.as_ref().map(|p| p.role) == Some(*r)
                }
                super::query::SquareContent::Occupied => {
                    piece_opt.is_some() || ep_piece_opt.is_some()
                }
                super::query::SquareContent::Empty => piece_opt.is_none() && ep_piece_opt.is_none(),
                super::query::SquareContent::AnyOf(pieces) => {
                    piece_opt
                        .as_ref()
                        .map(|p| pieces.contains(p))
                        .unwrap_or(false)
                        || ep_piece_opt
                            .as_ref()
                            .map(|p| pieces.contains(p))
                            .unwrap_or(false)
                }
                super::query::SquareContent::NoneOf(pieces) => piece_opt
                    .as_ref()
                    .map(|p| !pieces.contains(p))
                    .unwrap_or(true),
            });
            if !matches {
                return false;
            }
        }

        if let Some(role) = pattern.role {
            if mv.role() != role {
                return false;
            }
        }

        if let Some(color) = pattern.color {
            if pos_before.turn() != color {
                return false;
            }
        }

        if let Some(is_cap) = pattern.is_capture {
            if mv.is_capture() != is_cap {
                return false;
            }
        }

        if let Some(promotion_role) = pattern.promotion {
            if mv.promotion() != Some(promotion_role) {
                return false;
            }
        }

        if let Some(ref allowed_promotions) = pattern.promotions {
            match mv.promotion() {
                Some(promo) => {
                    if !allowed_promotions.contains(&promo) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        if let Some(chk) = pattern.is_check {
            if record.is_check != chk {
                return false;
            }
        }

        true
    }

    /// Match a legal move candidate available in the current position against a MovePattern
    pub fn match_legal_move(mv: &Move, pos_before: &Chess, pattern: &MovePattern) -> bool {
        if let Some(from_sq) = pattern.from {
            if mv.from() != Some(from_sq) {
                return false;
            }
        }

        if let Some(ref from_sqs) = pattern.from_squares {
            if let Some(from_sq) = mv.from() {
                if !from_sqs.contains(&from_sq) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref from_pcs) = pattern.from_pieces {
            if let Some(from_sq) = mv.from() {
                let piece_opt = pos_before.board().piece_at(from_sq);
                let matches = from_pcs.iter().any(|target| match target {
                    super::query::SquareContent::Piece(p) => piece_opt.as_ref() == Some(p),
                    super::query::SquareContent::Color(c) => {
                        piece_opt.as_ref().map(|p| p.color) == Some(*c)
                    }
                    super::query::SquareContent::Role(r) => {
                        piece_opt.as_ref().map(|p| p.role) == Some(*r)
                    }
                    super::query::SquareContent::Occupied => piece_opt.is_some(),
                    super::query::SquareContent::Empty => piece_opt.is_none(),
                    super::query::SquareContent::AnyOf(pieces) => piece_opt
                        .as_ref()
                        .map(|p| pieces.contains(p))
                        .unwrap_or(false),
                    super::query::SquareContent::NoneOf(pieces) => piece_opt
                        .as_ref()
                        .map(|p| !pieces.contains(p))
                        .unwrap_or(true),
                });
                if !matches {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(to_sq) = pattern.to {
            if !move_destination_matches(mv, to_sq) {
                return false;
            }
        }

        if let Some(ref to_sqs) = pattern.to_squares {
            if !move_destinations_contain(mv, to_sqs) {
                return false;
            }
        }

        if let Some(ref to_pcs) = pattern.to_pieces {
            let to_sq = if mv.is_castle() {
                match (pos_before.turn(), mv.to()) {
                    (shakmaty::Color::White, shakmaty::Square::A1) => shakmaty::Square::C1,
                    (shakmaty::Color::White, shakmaty::Square::H1) => shakmaty::Square::G1,
                    (shakmaty::Color::Black, shakmaty::Square::A8) => shakmaty::Square::C8,
                    (shakmaty::Color::Black, shakmaty::Square::H8) => shakmaty::Square::G8,
                    _ => mv.to(),
                }
            } else {
                mv.to()
            };
            let piece_opt = pos_before.board().piece_at(to_sq);
            let ep_piece_opt = if mv.is_en_passant() {
                mv.from().and_then(|from_sq| {
                    let ep_sq = shakmaty::Square::from_coords(to_sq.file(), from_sq.rank());
                    pos_before.board().piece_at(ep_sq)
                })
            } else {
                None
            };

            let matches = to_pcs.iter().any(|target| match target {
                super::query::SquareContent::Piece(p) => {
                    piece_opt.as_ref() == Some(p) || ep_piece_opt.as_ref() == Some(p)
                }
                super::query::SquareContent::Color(c) => {
                    piece_opt.as_ref().map(|p| p.color) == Some(*c)
                        || ep_piece_opt.as_ref().map(|p| p.color) == Some(*c)
                }
                super::query::SquareContent::Role(r) => {
                    piece_opt.as_ref().map(|p| p.role) == Some(*r)
                        || ep_piece_opt.as_ref().map(|p| p.role) == Some(*r)
                }
                super::query::SquareContent::Occupied => {
                    piece_opt.is_some() || ep_piece_opt.is_some()
                }
                super::query::SquareContent::Empty => piece_opt.is_none() && ep_piece_opt.is_none(),
                super::query::SquareContent::AnyOf(pieces) => {
                    piece_opt
                        .as_ref()
                        .map(|p| pieces.contains(p))
                        .unwrap_or(false)
                        || ep_piece_opt
                            .as_ref()
                            .map(|p| pieces.contains(p))
                            .unwrap_or(false)
                }
                super::query::SquareContent::NoneOf(pieces) => piece_opt
                    .as_ref()
                    .map(|p| !pieces.contains(p))
                    .unwrap_or(true),
            });
            if !matches {
                return false;
            }
        }

        if let Some(role) = pattern.role {
            if mv.role() != role {
                return false;
            }
        }

        if let Some(color) = pattern.color {
            if pos_before.turn() != color {
                return false;
            }
        }

        if let Some(is_cap) = pattern.is_capture {
            if mv.is_capture() != is_cap {
                return false;
            }
        }

        if let Some(promotion_role) = pattern.promotion {
            if mv.promotion() != Some(promotion_role) {
                return false;
            }
        }

        if let Some(ref allowed_promotions) = pattern.promotions {
            match mv.promotion() {
                Some(promo) => {
                    if !allowed_promotions.contains(&promo) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }

    /// Match a sequence of moves (PathPattern) against full game move history
    pub fn match_path(
        path: &PathPattern,
        positions_before: &[Chess],
        move_history: &[MoveRecord],
    ) -> Option<Vec<usize>> {
        if path.steps.is_empty() && path.moves.is_empty() {
            return Some(Vec::new());
        }

        let total_moves = move_history.len();

        let start_min = path.start_ply_range.as_ref().map(|r| r.start).unwrap_or(0);
        let start_max = path
            .start_ply_range
            .as_ref()
            .map(|r| r.end.min(total_moves))
            .unwrap_or(total_moves);

        // Normalize steps: if path.steps is empty, build steps from path.moves + path.consecutive / max_gap_plies
        let steps: Vec<super::query::PathStep> = if !path.steps.is_empty() {
            path.steps.clone()
        } else {
            let mut s = Vec::new();
            for (i, mv) in path.moves.iter().enumerate() {
                if i > 0 && !path.consecutive {
                    s.push(super::query::PathStep::Gap {
                        min: 0,
                        max: path.max_gap_plies,
                    });
                }
                s.push(super::query::PathStep::Move(mv.clone()));
            }
            s
        };

        for start_idx in start_min..start_max {
            if start_idx >= total_moves {
                break;
            }

            let mut matched_plies = Vec::new();
            if match_steps_from(
                &steps,
                0,
                start_idx,
                positions_before,
                move_history,
                &mut matched_plies,
            ) {
                return Some(matched_plies);
            }
        }

        None
    }
}

fn match_steps_from(
    steps: &[super::query::PathStep],
    step_idx: usize,
    cur_history_idx: usize,
    positions_before: &[Chess],
    move_history: &[MoveRecord],
    matched_plies: &mut Vec<usize>,
) -> bool {
    if step_idx >= steps.len() {
        return true;
    }

    match &steps[step_idx] {
        super::query::PathStep::Move(pattern) => {
            if cur_history_idx >= move_history.len() {
                return false;
            }
            if PathMatcher::match_move(
                pattern,
                &positions_before[cur_history_idx],
                &move_history[cur_history_idx],
            ) {
                matched_plies.push(move_history[cur_history_idx].ply);
                if match_steps_from(
                    steps,
                    step_idx + 1,
                    cur_history_idx + 1,
                    positions_before,
                    move_history,
                    matched_plies,
                ) {
                    return true;
                }
                matched_plies.pop();
            }
            false
        }
        super::query::PathStep::Gap { min, max } => {
            let min_gap = *min;
            let max_gap = max.unwrap_or(move_history.len().saturating_sub(cur_history_idx));

            for gap in min_gap..=max_gap {
                let next_history_idx = cur_history_idx + gap;
                if next_history_idx > move_history.len() {
                    break;
                }
                if match_steps_from(
                    steps,
                    step_idx + 1,
                    next_history_idx,
                    positions_before,
                    move_history,
                    matched_plies,
                ) {
                    return true;
                }
            }
            false
        }
    }
}

fn move_destination_matches(mv: &Move, target_sq: shakmaty::Square) -> bool {
    if mv.to() == target_sq {
        return true;
    }
    if mv.is_castle() {
        match (mv.from(), mv.to()) {
            (Some(shakmaty::Square::E1), shakmaty::Square::A1) => target_sq == shakmaty::Square::C1,
            (Some(shakmaty::Square::E1), shakmaty::Square::H1) => target_sq == shakmaty::Square::G1,
            (Some(shakmaty::Square::E8), shakmaty::Square::A8) => target_sq == shakmaty::Square::C8,
            (Some(shakmaty::Square::E8), shakmaty::Square::H8) => target_sq == shakmaty::Square::G8,
            _ => false,
        }
    } else {
        false
    }
}

fn move_destinations_contain(mv: &Move, to_sqs: &[shakmaty::Square]) -> bool {
    if to_sqs.contains(&mv.to()) {
        return true;
    }
    if mv.is_castle() {
        match (mv.from(), mv.to()) {
            (Some(shakmaty::Square::E1), shakmaty::Square::A1) => {
                to_sqs.contains(&shakmaty::Square::C1)
            }
            (Some(shakmaty::Square::E1), shakmaty::Square::H1) => {
                to_sqs.contains(&shakmaty::Square::G1)
            }
            (Some(shakmaty::Square::E8), shakmaty::Square::A8) => {
                to_sqs.contains(&shakmaty::Square::C8)
            }
            (Some(shakmaty::Square::E8), shakmaty::Square::H8) => {
                to_sqs.contains(&shakmaty::Square::G8)
            }
            _ => false,
        }
    } else {
        false
    }
}
