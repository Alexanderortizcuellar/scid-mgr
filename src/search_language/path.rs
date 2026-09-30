#![allow(
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_range_loop,
    clippy::int_plus_one
)]

use super::query::{CqlLinePattern, LineDirection, MovePattern, PathPattern};
use shakmaty::{Chess, Color, Move, Position};

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

        if let Some(ref cap_pcs) = pattern.captured_pieces {
            if !mv.is_capture() {
                return false;
            }
            let to_sq = mv.to();
            let piece_opt = pos_before.board().piece_at(to_sq);
            let ep_piece_opt = if mv.is_en_passant() {
                mv.from().and_then(|from_sq| {
                    let ep_sq = shakmaty::Square::from_coords(to_sq.file(), from_sq.rank());
                    pos_before.board().piece_at(ep_sq)
                })
            } else {
                None
            };
            let cap_piece = piece_opt.or(ep_piece_opt);
            let matches = cap_pcs.iter().any(|target| match target {
                super::query::SquareContent::Piece(p) => cap_piece.as_ref() == Some(p),
                super::query::SquareContent::Color(c) => {
                    cap_piece.as_ref().map(|p| p.color) == Some(*c)
                }
                super::query::SquareContent::Role(r) => {
                    cap_piece.as_ref().map(|p| p.role) == Some(*r)
                }
                super::query::SquareContent::Occupied => cap_piece.is_some(),
                super::query::SquareContent::Empty => cap_piece.is_none(),
                super::query::SquareContent::AnyOf(pieces) => cap_piece
                    .as_ref()
                    .map(|p| pieces.contains(p))
                    .unwrap_or(false),
                super::query::SquareContent::NoneOf(pieces) => cap_piece
                    .as_ref()
                    .map(|p| !pieces.contains(p))
                    .unwrap_or(true),
            });
            if !matches {
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

        if let Some(mate) = pattern.is_checkmate {
            let mut test_pos = pos_before.clone();
            test_pos.play_unchecked(&record.mv);
            if test_pos.is_checkmate() != mate {
                return false;
            }
        }

        if let Some(is_castle) = pattern.is_castle {
            if record.mv.is_castle() != is_castle {
                return false;
            }
        }

        if let Some(is_ep) = pattern.is_en_passant {
            if record.mv.is_en_passant() != is_ep {
                return false;
            }
        }

        if let Some((dir, min_dist, max_dist_opt)) = pattern.direction {
            let max_dist = max_dist_opt.unwrap_or(min_dist.max(7));
            if let Some(from_sq) = mv.from() {
                let to_sq = mv.to();
                let valid_sqs = dir.expand_square(from_sq, min_dist, max_dist);
                if !valid_sqs.contains(&to_sq) {
                    return false;
                }
            } else {
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

        if let Some(ref cap_pcs) = pattern.captured_pieces {
            if !mv.is_capture() {
                return false;
            }
            let to_sq = mv.to();
            let piece_opt = pos_before.board().piece_at(to_sq);
            let ep_piece_opt = if mv.is_en_passant() {
                mv.from().and_then(|from_sq| {
                    let ep_sq = shakmaty::Square::from_coords(to_sq.file(), from_sq.rank());
                    pos_before.board().piece_at(ep_sq)
                })
            } else {
                None
            };
            let cap_piece = piece_opt.or(ep_piece_opt);
            let matches = cap_pcs.iter().any(|target| match target {
                super::query::SquareContent::Piece(p) => cap_piece.as_ref() == Some(p),
                super::query::SquareContent::Color(c) => {
                    cap_piece.as_ref().map(|p| p.color) == Some(*c)
                }
                super::query::SquareContent::Role(r) => {
                    cap_piece.as_ref().map(|p| p.role) == Some(*r)
                }
                super::query::SquareContent::Occupied => cap_piece.is_some(),
                super::query::SquareContent::Empty => cap_piece.is_none(),
                super::query::SquareContent::AnyOf(pieces) => cap_piece
                    .as_ref()
                    .map(|p| pieces.contains(p))
                    .unwrap_or(false),
                super::query::SquareContent::NoneOf(pieces) => cap_piece
                    .as_ref()
                    .map(|p| !pieces.contains(p))
                    .unwrap_or(true),
            });
            if !matches {
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

        if let Some(is_chk) = pattern.is_check {
            let mut test_pos = pos_before.clone();
            test_pos.play_unchecked(mv);
            if test_pos.is_check() != is_chk {
                return false;
            }
        }

        if let Some(is_mate) = pattern.is_checkmate {
            let mut test_pos = pos_before.clone();
            test_pos.play_unchecked(mv);
            if test_pos.is_checkmate() != is_mate {
                return false;
            }
        }

        if let Some(is_castle) = pattern.is_castle {
            if mv.is_castle() != is_castle {
                return false;
            }
        }

        if let Some(is_ep) = pattern.is_en_passant {
            if mv.is_en_passant() != is_ep {
                return false;
            }
        }

        if let Some((dir, min_dist, max_dist_opt)) = pattern.direction {
            let max_dist = max_dist_opt.unwrap_or(min_dist.max(7));
            if let Some(from_sq) = mv.from() {
                let to_sq = mv.to();
                let valid_sqs = dir.expand_square(from_sq, min_dist, max_dist);
                if !valid_sqs.contains(&to_sq) {
                    return false;
                }
            } else {
                return false;
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
                s.push(super::query::PathStep::Move {
                    pattern: mv.clone(),
                    repeat_min: 1,
                    repeat_max: Some(1),
                });
            }
            s
        };

        for start_idx in start_min..start_max {
            if start_idx >= total_moves {
                break;
            }

            // If single_color is specified with a fixed color, verify the start turn
            if let Some(Some(required_color)) = path.single_color {
                if positions_before[start_idx].turn() != required_color {
                    continue;
                }
            }

            let mut matched_plies = Vec::new();
            if match_steps_from(
                &steps,
                0,
                start_idx,
                path.single_color,
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
    single_color: Option<Option<Color>>,
    positions_before: &[Chess],
    move_history: &[MoveRecord],
    matched_plies: &mut Vec<usize>,
) -> bool {
    if step_idx >= steps.len() {
        return true;
    }

    match &steps[step_idx] {
        super::query::PathStep::Move {
            pattern,
            repeat_min,
            repeat_max,
        } => {
            let min_reps = *repeat_min;
            let max_reps = repeat_max.unwrap_or(move_history.len().saturating_sub(cur_history_idx));

            // Backtracking search over possible match repetitions of this move pattern
            // single_color step delta: 2 plies if single_color is enabled, else 1 ply
            let step_ply_delta = if single_color.is_some() { 2 } else { 1 };

            // Find how many consecutive matches are possible starting at cur_history_idx
            let mut candidate_idxs = Vec::new();
            let mut idx = cur_history_idx;
            let mut color_context = single_color;

            while idx < move_history.len() && candidate_idxs.len() < max_reps {
                // If single_color was Some(None) (auto-detect), fix it from the first matched move
                if let Some(None) = color_context {
                    let move_color = positions_before[idx].turn();
                    color_context = Some(Some(move_color));
                }

                if let Some(Some(expected_color)) = color_context {
                    if positions_before[idx].turn() != expected_color {
                        break;
                    }
                }

                if PathMatcher::match_move(pattern, &positions_before[idx], &move_history[idx]) {
                    candidate_idxs.push(idx);
                    idx += step_ply_delta;
                } else {
                    break;
                }
            }

            // Try branches from highest matched count down to min_reps (greedy with backtracking)
            for count in (min_reps..=candidate_idxs.len()).rev() {
                let prev_len = matched_plies.len();
                for &c_idx in &candidate_idxs[..count] {
                    matched_plies.push(move_history[c_idx].ply);
                }

                let next_history_idx = if count == 0 {
                    cur_history_idx
                } else {
                    candidate_idxs[count - 1] + step_ply_delta
                };

                if match_steps_from(
                    steps,
                    step_idx + 1,
                    next_history_idx,
                    color_context,
                    positions_before,
                    move_history,
                    matched_plies,
                ) {
                    return true;
                }

                matched_plies.truncate(prev_len);
            }

            false
        }
        super::query::PathStep::Gap { min, max } => {
            let min_gap = *min;
            let max_gap = max.unwrap_or(move_history.len().saturating_sub(cur_history_idx));
            let gap_multiplier = if single_color.is_some() { 2 } else { 1 };

            for gap in min_gap..=max_gap {
                let next_history_idx = cur_history_idx + gap * gap_multiplier;
                if next_history_idx > move_history.len() {
                    break;
                }
                if match_steps_from(
                    steps,
                    step_idx + 1,
                    next_history_idx,
                    single_color,
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

/// Evaluates CQL 6.2 CqlPathPattern with interleaved moves and positional state filters
pub struct CqlPathMatcher;

impl CqlPathMatcher {
    /// Match a CqlPathPattern against full game move history and positions
    pub fn match_cql_path(
        path: &super::query::CqlPathPattern,
        positions_before: &[Chess],
        move_history: &[MoveRecord],
    ) -> Option<Vec<usize>> {
        if path.constituents.is_empty() {
            return Some(Vec::new());
        }

        let total_positions = positions_before.len();
        let start_min = path.start_ply_range.as_ref().map(|r| r.start).unwrap_or(0);
        let start_max = path
            .start_ply_range
            .as_ref()
            .map(|r| r.end.min(total_positions))
            .unwrap_or(total_positions);

        for start_idx in start_min..start_max {
            if start_idx >= total_positions {
                break;
            }

            if let Some(Some(required_color)) = path.single_color {
                if positions_before[start_idx].turn() != required_color {
                    continue;
                }
            }

            let mut matched_plies = Vec::new();
            if match_cql_constituents_from(
                &path.constituents,
                0,
                start_idx,
                path.single_color,
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

fn match_cql_constituents_from(
    constituents: &[super::query::CqlPathConstituent],
    const_idx: usize,
    cur_pos_idx: usize,
    single_color: Option<Option<Color>>,
    positions_before: &[Chess],
    move_history: &[MoveRecord],
    matched_plies: &mut Vec<usize>,
) -> bool {
    if const_idx >= constituents.len() {
        return true;
    }

    match &constituents[const_idx] {
        super::query::CqlPathConstituent::Filter(query) => {
            if cur_pos_idx >= positions_before.len() {
                return false;
            }
            let pos = &positions_before[cur_pos_idx];
            let last_move = if cur_pos_idx > 0 && cur_pos_idx - 1 < move_history.len() {
                Some((
                    &positions_before[cur_pos_idx - 1],
                    &move_history[cur_pos_idx - 1],
                ))
            } else {
                None
            };

            if crate::search::evaluator::matcher::matches_single_ply(
                query,
                pos,
                cur_pos_idx,
                last_move,
            ) {
                match_cql_constituents_from(
                    constituents,
                    const_idx + 1,
                    cur_pos_idx,
                    single_color,
                    positions_before,
                    move_history,
                    matched_plies,
                )
            } else {
                false
            }
        }
        super::query::CqlPathConstituent::Move(pattern) => {
            if cur_pos_idx >= move_history.len() || cur_pos_idx >= positions_before.len() {
                return false;
            }

            let pos_before = &positions_before[cur_pos_idx];
            let record = &move_history[cur_pos_idx];

            if PathMatcher::match_move(pattern, pos_before, record) {
                let step_ply_delta = if single_color.is_some() { 2 } else { 1 };
                let prev_len = matched_plies.len();
                matched_plies.push(record.ply);

                if match_cql_constituents_from(
                    constituents,
                    const_idx + 1,
                    cur_pos_idx + step_ply_delta,
                    single_color,
                    positions_before,
                    move_history,
                    matched_plies,
                ) {
                    return true;
                }

                matched_plies.truncate(prev_len);
            }

            false
        }
        super::query::CqlPathConstituent::Chain(subs) => {
            let mut expanded = subs.clone();
            expanded.extend_from_slice(&constituents[const_idx + 1..]);
            match_cql_constituents_from(
                &expanded,
                0,
                cur_pos_idx,
                single_color,
                positions_before,
                move_history,
                matched_plies,
            )
        }
        super::query::CqlPathConstituent::Repetition {
            constituent,
            min,
            max,
        } => {
            let min_reps = *min;
            let max_reps = max.unwrap_or(20);

            // Backtracking search: try matching 0 to max_reps repetitions of `constituent`
            fn match_rep_helper(
                inner: &super::query::CqlPathConstituent,
                remaining_constituents: &[super::query::CqlPathConstituent],
                reps_done: usize,
                min_reps: usize,
                max_reps: usize,
                cur_pos_idx: usize,
                single_color: Option<Option<Color>>,
                positions_before: &[Chess],
                move_history: &[MoveRecord],
                matched_plies: &mut Vec<usize>,
            ) -> bool {
                if reps_done >= min_reps {
                    let prev_len = matched_plies.len();
                    if match_cql_constituents_from(
                        remaining_constituents,
                        0,
                        cur_pos_idx,
                        single_color,
                        positions_before,
                        move_history,
                        matched_plies,
                    ) {
                        return true;
                    }
                    matched_plies.truncate(prev_len);
                }

                if reps_done < max_reps {
                    let prev_len = matched_plies.len();
                    // We try to match one iteration and get the next pos idx
                    // For single move or chain
                    match inner {
                        super::query::CqlPathConstituent::Move(pattern) => {
                            if cur_pos_idx < move_history.len()
                                && cur_pos_idx < positions_before.len()
                            {
                                if PathMatcher::match_move(
                                    pattern,
                                    &positions_before[cur_pos_idx],
                                    &move_history[cur_pos_idx],
                                ) {
                                    matched_plies.push(move_history[cur_pos_idx].ply);
                                    let step_delta = if single_color.is_some() { 2 } else { 1 };
                                    if match_rep_helper(
                                        inner,
                                        remaining_constituents,
                                        reps_done + 1,
                                        min_reps,
                                        max_reps,
                                        cur_pos_idx + step_delta,
                                        single_color,
                                        positions_before,
                                        move_history,
                                        matched_plies,
                                    ) {
                                        return true;
                                    }
                                }
                            }
                        }
                        super::query::CqlPathConstituent::Chain(subs) => {
                            // Match chain sequentially
                            let mut chain_end_idx = cur_pos_idx;
                            let mut chain_ok = true;
                            let chain_prev_len = matched_plies.len();
                            for sub in subs {
                                match sub {
                                    super::query::CqlPathConstituent::Filter(q) => {
                                        if chain_end_idx >= positions_before.len() {
                                            chain_ok = false;
                                            break;
                                        }
                                        let last_mv = if chain_end_idx > 0
                                            && chain_end_idx - 1 < move_history.len()
                                        {
                                            Some((
                                                &positions_before[chain_end_idx - 1],
                                                &move_history[chain_end_idx - 1],
                                            ))
                                        } else {
                                            None
                                        };
                                        if !crate::search::evaluator::matcher::matches_single_ply(
                                            q,
                                            &positions_before[chain_end_idx],
                                            chain_end_idx,
                                            last_mv,
                                        ) {
                                            chain_ok = false;
                                            break;
                                        }
                                    }
                                    super::query::CqlPathConstituent::Move(pat) => {
                                        if chain_end_idx >= move_history.len()
                                            || chain_end_idx >= positions_before.len()
                                        {
                                            chain_ok = false;
                                            break;
                                        }
                                        if PathMatcher::match_move(
                                            pat,
                                            &positions_before[chain_end_idx],
                                            &move_history[chain_end_idx],
                                        ) {
                                            matched_plies.push(move_history[chain_end_idx].ply);
                                            let step_delta =
                                                if single_color.is_some() { 2 } else { 1 };
                                            chain_end_idx += step_delta;
                                        } else {
                                            chain_ok = false;
                                            break;
                                        }
                                    }
                                    _ => {
                                        chain_ok = false;
                                        break;
                                    }
                                }
                            }

                            if chain_ok && chain_end_idx > cur_pos_idx {
                                if match_rep_helper(
                                    inner,
                                    remaining_constituents,
                                    reps_done + 1,
                                    min_reps,
                                    max_reps,
                                    chain_end_idx,
                                    single_color,
                                    positions_before,
                                    move_history,
                                    matched_plies,
                                ) {
                                    return true;
                                }
                            }
                            matched_plies.truncate(chain_prev_len);
                        }
                        _ => {}
                    }
                    matched_plies.truncate(prev_len);
                }

                false
            }

            match_rep_helper(
                constituent,
                &constituents[const_idx + 1..],
                0,
                min_reps,
                max_reps,
                cur_pos_idx,
                single_color,
                positions_before,
                move_history,
                matched_plies,
            )
        }
    }
}

/// Evaluates CQLi CqlLinePattern with position and move transitions along arrows
pub struct CqlLineMatcher;

impl CqlLineMatcher {
    /// Match a CqlLinePattern against full game move history and positions
    pub fn match_cql_line(
        line: &CqlLinePattern,
        positions_before: &[Chess],
        move_history: &[MoveRecord],
    ) -> Option<Vec<usize>> {
        if line.constituents.is_empty() {
            if let Some(min_l) = line.min_length {
                if min_l > 0 {
                    return None;
                }
            }
            return Some(Vec::new());
        }

        let total_positions = positions_before.len();
        if total_positions == 0 {
            return None;
        }

        let start_min = line.start_ply_range.as_ref().map(|r| r.start).unwrap_or(0);
        let start_max = line
            .start_ply_range
            .as_ref()
            .map(|r| r.end.min(total_positions))
            .unwrap_or(total_positions);

        let step_delta = if line.single_color.is_some() { 2 } else { 1 };
        let mut banned_plies = vec![false; total_positions];
        let mut matched_results = Vec::new();

        for start_idx in start_min..start_max {
            if start_idx >= total_positions {
                break;
            }

            if line.nest_ban && banned_plies[start_idx] {
                continue;
            }

            if let Some(Some(required_color)) = line.single_color {
                if positions_before[start_idx].turn() != required_color {
                    continue;
                }
            }

            let match_opt = match line.direction {
                LineDirection::Forward => match_line_forward_from(
                    &line.constituents,
                    0,
                    start_idx,
                    step_delta,
                    positions_before,
                    move_history,
                    0,
                ),
                LineDirection::Backward => match_line_backward_from(
                    &line.constituents,
                    0,
                    start_idx,
                    step_delta,
                    positions_before,
                    move_history,
                    0,
                ),
            };

            if let Some((total_steps, tail_idx)) = match_opt {
                if let Some(min_l) = line.min_length {
                    if total_steps < min_l {
                        continue;
                    }
                }
                if let Some(max_l) = line.max_length {
                    if total_steps > max_l {
                        continue;
                    }
                }
                if line.min_length.is_none() && line.max_length.is_none() && total_steps == 0 {
                    continue;
                }

                if line.last_position {
                    matched_results.push(tail_idx);
                } else {
                    matched_results.push(start_idx);
                }

                if line.nest_ban {
                    let low = start_idx.min(tail_idx);
                    let high = start_idx.max(tail_idx);
                    for p in low..=high.min(total_positions.saturating_sub(1)) {
                        banned_plies[p] = true;
                    }
                }

                if line.first_match {
                    return Some(matched_results);
                }
            }
        }

        if !matched_results.is_empty() {
            Some(matched_results)
        } else {
            None
        }
    }
}

fn match_line_forward_from(
    constituents: &[super::query::CqlPathConstituent],
    const_idx: usize,
    cur_pos_idx: usize,
    step_delta: usize,
    positions_before: &[Chess],
    move_history: &[MoveRecord],
    steps_count: usize,
) -> Option<(usize, usize)> {
    if const_idx >= constituents.len() {
        let tail_idx = if steps_count > 0 {
            cur_pos_idx.saturating_sub(step_delta)
        } else {
            cur_pos_idx
        };
        return Some((steps_count, tail_idx));
    }

    match &constituents[const_idx] {
        super::query::CqlPathConstituent::Filter(query) => {
            if cur_pos_idx >= positions_before.len() {
                return None;
            }
            let pos = &positions_before[cur_pos_idx];
            let last_move = if cur_pos_idx > 0 && cur_pos_idx - 1 < move_history.len() {
                Some((
                    &positions_before[cur_pos_idx - 1],
                    &move_history[cur_pos_idx - 1],
                ))
            } else {
                None
            };

            if crate::search::evaluator::matcher::matches_single_ply(
                query,
                pos,
                cur_pos_idx,
                last_move,
            ) {
                if const_idx + 1 >= constituents.len() {
                    return Some((steps_count + 1, cur_pos_idx));
                }
                match_line_forward_from(
                    constituents,
                    const_idx + 1,
                    cur_pos_idx + step_delta,
                    step_delta,
                    positions_before,
                    move_history,
                    steps_count + 1,
                )
            } else {
                None
            }
        }
        super::query::CqlPathConstituent::Move(pattern) => {
            if cur_pos_idx >= move_history.len() || cur_pos_idx >= positions_before.len() {
                return None;
            }
            let pos_before = &positions_before[cur_pos_idx];
            let record = &move_history[cur_pos_idx];

            if PathMatcher::match_move(pattern, pos_before, record) {
                if const_idx + 1 >= constituents.len() {
                    return Some((steps_count + 1, cur_pos_idx));
                }
                match_line_forward_from(
                    constituents,
                    const_idx + 1,
                    cur_pos_idx + step_delta,
                    step_delta,
                    positions_before,
                    move_history,
                    steps_count + 1,
                )
            } else {
                None
            }
        }
        super::query::CqlPathConstituent::Chain(subs) => {
            let mut expanded = subs.clone();
            expanded.extend_from_slice(&constituents[const_idx + 1..]);
            match_line_forward_from(
                &expanded,
                0,
                cur_pos_idx,
                step_delta,
                positions_before,
                move_history,
                steps_count,
            )
        }
        super::query::CqlPathConstituent::Repetition {
            constituent,
            min,
            max,
        } => {
            let min_reps = *min;
            let max_reps = max.unwrap_or(100);

            fn try_rep_forward(
                inner: &super::query::CqlPathConstituent,
                remaining_constituents: &[super::query::CqlPathConstituent],
                reps_done: usize,
                min_reps: usize,
                max_reps: usize,
                cur_pos_idx: usize,
                step_delta: usize,
                positions_before: &[Chess],
                move_history: &[MoveRecord],
                steps_count: usize,
            ) -> Option<(usize, usize)> {
                let mut best_res: Option<(usize, usize)> = None;

                if reps_done < max_reps {
                    match inner {
                        super::query::CqlPathConstituent::Filter(q) => {
                            if cur_pos_idx < positions_before.len() {
                                let last_mv =
                                    if cur_pos_idx > 0 && cur_pos_idx - 1 < move_history.len() {
                                        Some((
                                            &positions_before[cur_pos_idx - 1],
                                            &move_history[cur_pos_idx - 1],
                                        ))
                                    } else {
                                        None
                                    };
                                if crate::search::evaluator::matcher::matches_single_ply(
                                    q,
                                    &positions_before[cur_pos_idx],
                                    cur_pos_idx,
                                    last_mv,
                                ) {
                                    if let Some(res) = try_rep_forward(
                                        inner,
                                        remaining_constituents,
                                        reps_done + 1,
                                        min_reps,
                                        max_reps,
                                        cur_pos_idx + step_delta,
                                        step_delta,
                                        positions_before,
                                        move_history,
                                        steps_count + 1,
                                    ) {
                                        if best_res.is_none()
                                            || res.0 > best_res.as_ref().unwrap().0
                                        {
                                            best_res = Some(res);
                                        }
                                    }
                                }
                            }
                        }
                        super::query::CqlPathConstituent::Move(pat) => {
                            if cur_pos_idx < move_history.len()
                                && cur_pos_idx < positions_before.len()
                            {
                                if PathMatcher::match_move(
                                    pat,
                                    &positions_before[cur_pos_idx],
                                    &move_history[cur_pos_idx],
                                ) {
                                    if let Some(res) = try_rep_forward(
                                        inner,
                                        remaining_constituents,
                                        reps_done + 1,
                                        min_reps,
                                        max_reps,
                                        cur_pos_idx + step_delta,
                                        step_delta,
                                        positions_before,
                                        move_history,
                                        steps_count + 1,
                                    ) {
                                        if best_res.is_none()
                                            || res.0 > best_res.as_ref().unwrap().0
                                        {
                                            best_res = Some(res);
                                        }
                                    }
                                }
                            }
                        }
                        super::query::CqlPathConstituent::Chain(subs) => {
                            let mut chain_end_idx = cur_pos_idx;
                            let mut chain_steps = 0;
                            let mut chain_ok = true;
                            for sub in subs {
                                match sub {
                                    super::query::CqlPathConstituent::Filter(q) => {
                                        if chain_end_idx >= positions_before.len() {
                                            chain_ok = false;
                                            break;
                                        }
                                        let last_mv = if chain_end_idx > 0
                                            && chain_end_idx - 1 < move_history.len()
                                        {
                                            Some((
                                                &positions_before[chain_end_idx - 1],
                                                &move_history[chain_end_idx - 1],
                                            ))
                                        } else {
                                            None
                                        };
                                        if crate::search::evaluator::matcher::matches_single_ply(
                                            q,
                                            &positions_before[chain_end_idx],
                                            chain_end_idx,
                                            last_mv,
                                        ) {
                                            chain_end_idx += step_delta;
                                            chain_steps += 1;
                                        } else {
                                            chain_ok = false;
                                            break;
                                        }
                                    }
                                    super::query::CqlPathConstituent::Move(pat) => {
                                        if chain_end_idx >= move_history.len()
                                            || chain_end_idx >= positions_before.len()
                                        {
                                            chain_ok = false;
                                            break;
                                        }
                                        if PathMatcher::match_move(
                                            pat,
                                            &positions_before[chain_end_idx],
                                            &move_history[chain_end_idx],
                                        ) {
                                            chain_end_idx += step_delta;
                                            chain_steps += 1;
                                        } else {
                                            chain_ok = false;
                                            break;
                                        }
                                    }
                                    _ => {
                                        chain_ok = false;
                                        break;
                                    }
                                }
                            }
                            if chain_ok && chain_steps > 0 {
                                if let Some(res) = try_rep_forward(
                                    inner,
                                    remaining_constituents,
                                    reps_done + 1,
                                    min_reps,
                                    max_reps,
                                    chain_end_idx,
                                    step_delta,
                                    positions_before,
                                    move_history,
                                    steps_count + chain_steps,
                                ) {
                                    if best_res.is_none() || res.0 > best_res.as_ref().unwrap().0 {
                                        best_res = Some(res);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if reps_done >= min_reps {
                    if let Some(res) = match_line_forward_from(
                        remaining_constituents,
                        0,
                        cur_pos_idx,
                        step_delta,
                        positions_before,
                        move_history,
                        steps_count,
                    ) {
                        if best_res.is_none() || res.0 > best_res.as_ref().unwrap().0 {
                            best_res = Some(res);
                        }
                    }
                }

                best_res
            }

            try_rep_forward(
                constituent,
                &constituents[const_idx + 1..],
                0,
                min_reps,
                max_reps,
                cur_pos_idx,
                step_delta,
                positions_before,
                move_history,
                steps_count,
            )
        }
    }
}

fn match_line_backward_from(
    constituents: &[super::query::CqlPathConstituent],
    const_idx: usize,
    cur_pos_idx: usize,
    step_delta: usize,
    positions_before: &[Chess],
    move_history: &[MoveRecord],
    steps_count: usize,
) -> Option<(usize, usize)> {
    if const_idx >= constituents.len() {
        let tail_idx = if steps_count > 0 {
            (cur_pos_idx + step_delta).min(positions_before.len() - 1)
        } else {
            cur_pos_idx
        };
        return Some((steps_count, tail_idx));
    }

    match &constituents[const_idx] {
        super::query::CqlPathConstituent::Filter(query) => {
            if cur_pos_idx >= positions_before.len() {
                return None;
            }
            let pos = &positions_before[cur_pos_idx];
            let last_move = if cur_pos_idx > 0 && cur_pos_idx - 1 < move_history.len() {
                Some((
                    &positions_before[cur_pos_idx - 1],
                    &move_history[cur_pos_idx - 1],
                ))
            } else {
                None
            };

            if crate::search::evaluator::matcher::matches_single_ply(
                query,
                pos,
                cur_pos_idx,
                last_move,
            ) {
                if const_idx + 1 >= constituents.len() {
                    return Some((steps_count + 1, cur_pos_idx));
                }
                if cur_pos_idx >= step_delta {
                    match_line_backward_from(
                        constituents,
                        const_idx + 1,
                        cur_pos_idx - step_delta,
                        step_delta,
                        positions_before,
                        move_history,
                        steps_count + 1,
                    )
                } else {
                    None
                }
            } else {
                None
            }
        }
        super::query::CqlPathConstituent::Move(pattern) => {
            if cur_pos_idx < 1
                || cur_pos_idx - 1 >= move_history.len()
                || cur_pos_idx - 1 >= positions_before.len()
            {
                return None;
            }
            let pos_before = &positions_before[cur_pos_idx - 1];
            let record = &move_history[cur_pos_idx - 1];

            if PathMatcher::match_move(pattern, pos_before, record) {
                if const_idx + 1 >= constituents.len() {
                    return Some((steps_count + 1, cur_pos_idx - 1));
                }
                if cur_pos_idx >= step_delta {
                    match_line_backward_from(
                        constituents,
                        const_idx + 1,
                        cur_pos_idx - step_delta,
                        step_delta,
                        positions_before,
                        move_history,
                        steps_count + 1,
                    )
                } else {
                    None
                }
            } else {
                None
            }
        }
        super::query::CqlPathConstituent::Chain(subs) => {
            let mut expanded = subs.clone();
            expanded.extend_from_slice(&constituents[const_idx + 1..]);
            match_line_backward_from(
                &expanded,
                0,
                cur_pos_idx,
                step_delta,
                positions_before,
                move_history,
                steps_count,
            )
        }
        super::query::CqlPathConstituent::Repetition {
            constituent,
            min,
            max,
        } => {
            let min_reps = *min;
            let max_reps = max.unwrap_or(100);

            fn try_rep_backward(
                inner: &super::query::CqlPathConstituent,
                remaining_constituents: &[super::query::CqlPathConstituent],
                reps_done: usize,
                min_reps: usize,
                max_reps: usize,
                cur_pos_idx: usize,
                step_delta: usize,
                positions_before: &[Chess],
                move_history: &[MoveRecord],
                steps_count: usize,
            ) -> Option<(usize, usize)> {
                let mut best_res: Option<(usize, usize)> = None;

                if reps_done < max_reps {
                    match inner {
                        super::query::CqlPathConstituent::Filter(q) => {
                            if cur_pos_idx < positions_before.len() {
                                let last_mv =
                                    if cur_pos_idx > 0 && cur_pos_idx - 1 < move_history.len() {
                                        Some((
                                            &positions_before[cur_pos_idx - 1],
                                            &move_history[cur_pos_idx - 1],
                                        ))
                                    } else {
                                        None
                                    };
                                if crate::search::evaluator::matcher::matches_single_ply(
                                    q,
                                    &positions_before[cur_pos_idx],
                                    cur_pos_idx,
                                    last_mv,
                                ) {
                                    let next_idx_opt = if cur_pos_idx >= step_delta {
                                        Some(cur_pos_idx - step_delta)
                                    } else {
                                        None
                                    };
                                    if let Some(next_idx) = next_idx_opt {
                                        if let Some(res) = try_rep_backward(
                                            inner,
                                            remaining_constituents,
                                            reps_done + 1,
                                            min_reps,
                                            max_reps,
                                            next_idx,
                                            step_delta,
                                            positions_before,
                                            move_history,
                                            steps_count + 1,
                                        ) {
                                            if best_res.is_none()
                                                || res.0 > best_res.as_ref().unwrap().0
                                            {
                                                best_res = Some(res);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        super::query::CqlPathConstituent::Move(pat) => {
                            if cur_pos_idx >= 1
                                && cur_pos_idx - 1 < move_history.len()
                                && cur_pos_idx - 1 < positions_before.len()
                            {
                                if PathMatcher::match_move(
                                    pat,
                                    &positions_before[cur_pos_idx - 1],
                                    &move_history[cur_pos_idx - 1],
                                ) {
                                    let next_idx_opt = if cur_pos_idx >= step_delta {
                                        Some(cur_pos_idx - step_delta)
                                    } else {
                                        None
                                    };
                                    if let Some(next_idx) = next_idx_opt {
                                        if let Some(res) = try_rep_backward(
                                            inner,
                                            remaining_constituents,
                                            reps_done + 1,
                                            min_reps,
                                            max_reps,
                                            next_idx,
                                            step_delta,
                                            positions_before,
                                            move_history,
                                            steps_count + 1,
                                        ) {
                                            if best_res.is_none()
                                                || res.0 > best_res.as_ref().unwrap().0
                                            {
                                                best_res = Some(res);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if reps_done >= min_reps {
                    if let Some(res) = match_line_backward_from(
                        remaining_constituents,
                        0,
                        cur_pos_idx,
                        step_delta,
                        positions_before,
                        move_history,
                        steps_count,
                    ) {
                        if best_res.is_none() || res.0 > best_res.as_ref().unwrap().0 {
                            best_res = Some(res);
                        }
                    }
                }

                best_res
            }

            try_rep_backward(
                constituent,
                &constituents[const_idx + 1..],
                0,
                min_reps,
                max_reps,
                cur_pos_idx,
                step_delta,
                positions_before,
                move_history,
                steps_count,
            )
        }
    }
}
