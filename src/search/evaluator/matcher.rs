use shakmaty::{Chess, Position};
use std::collections::HashMap;

use super::QueryMatchResult;
use crate::search::header::HeaderMatcher;
use crate::search::path::{MoveRecord, PathMatcher};
use crate::search::pattern::PositionMatcher;
use crate::search::pawn::PawnEvaluator;
use crate::search::query::SearchQuery;
use crate::search::tactics::TacticsEvaluator;

/// Evaluate a SearchQuery AST across game timeline positions and moves
pub fn evaluate_with_timeline_env(
    query: &SearchQuery,
    headers: &HashMap<String, String>,
    positions: &[Chess],
    moves: &[MoveRecord],
    env: &HashMap<String, shakmaty::Square>,
) -> QueryMatchResult {
    match query {
        SearchQuery::Header(pred) => {
            let matches = HeaderMatcher::matches(pred, headers);
            QueryMatchResult {
                is_match: matches,
                matching_plies: if matches { vec![0] } else { Vec::new() },
                match_count: if matches { 1 } else { 0 },
            }
        }
        SearchQuery::Position(pattern) => {
            let mut matched_plies = Vec::new();
            for (ply, pos) in positions.iter().enumerate() {
                if PositionMatcher::matches_at_ply_with_env(pattern, pos, ply, env) {
                    matched_plies.push(ply);
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Pawn(pred) => {
            let mut matched_plies = Vec::new();
            for (ply, pos) in positions.iter().enumerate() {
                if PawnEvaluator::matches(pred, pos.board()) {
                    matched_plies.push(ply);
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Tactical(pred) => {
            let mut matched_plies = Vec::new();
            for (ply, pos) in positions.iter().enumerate() {
                if TacticsEvaluator::matches_with_env(pred, pos, env) {
                    matched_plies.push(ply);
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::VariableBinding {
            var_name,
            domain,
            query: sub_query,
        } => {
            let mut matched_plies = Vec::new();
            for (ply, pos) in positions.iter().enumerate() {
                let candidates: Vec<shakmaty::Square> = match domain {
                    crate::search::query::VariableDomain::Piece(matcher) => {
                        let mut sqs = Vec::new();
                        for sq in shakmaty::Square::ALL {
                            if let Some(piece) = pos.board().piece_at(sq) {
                                if matcher.matches(&piece) {
                                    sqs.push(sq);
                                }
                            }
                        }
                        sqs
                    }
                    crate::search::query::VariableDomain::SquareSet(sqs) => sqs.clone(),
                    crate::search::query::VariableDomain::AnyPiece => {
                        pos.board().occupied().into_iter().collect()
                    }
                };

                let current_ply_moves: Vec<MoveRecord> =
                    moves.iter().filter(|m| m.ply == ply).cloned().collect();

                let mut any_candidate_matched = false;
                for cand_sq in candidates {
                    let mut sub_env = env.clone();
                    sub_env.insert(var_name.clone(), cand_sq);
                    let sub_res = evaluate_with_timeline_env(
                        sub_query,
                        headers,
                        std::slice::from_ref(pos),
                        &current_ply_moves,
                        &sub_env,
                    );
                    if sub_res.is_match {
                        any_candidate_matched = true;
                        break;
                    }
                }
                if any_candidate_matched {
                    matched_plies.push(ply);
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Material(pred) => {
            let mut matched_plies = Vec::new();
            for (ply, pos) in positions.iter().enumerate() {
                if PositionMatcher::matches_material(pred, pos) {
                    matched_plies.push(ply);
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Power(pred) => {
            let mut matched_plies = Vec::new();
            for (ply, pos) in positions.iter().enumerate() {
                if PositionMatcher::matches_power(pred, pos) {
                    matched_plies.push(ply);
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Move(move_pattern) => {
            let mut matched_plies = Vec::new();
            if move_pattern.is_legal || move_pattern.count_predicate.is_some() {
                for (ply, pos) in positions.iter().enumerate() {
                    let legal_moves = pos.legal_moves();
                    let matching_count = legal_moves
                        .iter()
                        .filter(|m| PathMatcher::match_legal_move(m, pos, move_pattern))
                        .count();
                    let is_pos_match = if let Some((op, count)) = move_pattern.count_predicate {
                        match op {
                            crate::search::query::ComparisonOp::Equal => matching_count == count,
                            crate::search::query::ComparisonOp::NotEqual => matching_count != count,
                            crate::search::query::ComparisonOp::GreaterThan => {
                                matching_count > count
                            }
                            crate::search::query::ComparisonOp::GreaterThanOrEqual => {
                                matching_count >= count
                            }
                            crate::search::query::ComparisonOp::LessThan => matching_count < count,
                            crate::search::query::ComparisonOp::LessThanOrEqual => {
                                matching_count <= count
                            }
                            _ => matching_count == count,
                        }
                    } else {
                        matching_count > 0
                    };
                    if is_pos_match {
                        matched_plies.push(ply);
                    }
                }
            } else {
                for (idx, record) in moves.iter().enumerate() {
                    if idx < positions.len()
                        && PathMatcher::match_move(move_pattern, &positions[idx], record)
                    {
                        matched_plies.push(record.ply);
                    }
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Path(path_pattern) => {
            let matched = PathMatcher::match_path(path_pattern, positions, moves);
            match matched {
                Some(plies) => QueryMatchResult {
                    is_match: true,
                    matching_plies: plies,
                    match_count: 1,
                },
                None => QueryMatchResult::default(),
            }
        }
        SearchQuery::PlyRange { range, query } => {
            let start_ply = range.start;
            let end_ply = range.end.min(positions.len());

            if start_ply >= positions.len() || start_ply >= end_ply {
                return QueryMatchResult::default();
            }

            let sub_positions = &positions[start_ply..end_ply];
            let sub_moves: Vec<MoveRecord> = moves
                .iter()
                .filter(|m| m.ply >= start_ply && m.ply < end_ply)
                .cloned()
                .collect();

            evaluate_with_timeline_env(query, headers, sub_positions, &sub_moves, env)
        }
        SearchQuery::Occurrences { min, max, query } => {
            let sub_res = evaluate_with_timeline_env(query, headers, positions, moves, env);
            let count = sub_res.match_count;
            let mut meets = count >= *min;
            if let Some(max_val) = max {
                if count > *max_val {
                    meets = false;
                }
            }
            QueryMatchResult {
                is_match: meets,
                matching_plies: if meets {
                    sub_res.matching_plies
                } else {
                    Vec::new()
                },
                match_count: count,
            }
        }
        SearchQuery::And(sub_queries) => {
            let mut is_first_positional = true;
            let mut matching_plies_set: Vec<usize> = Vec::new();
            let mut has_positional = false;

            for q in sub_queries {
                let res = match q {
                    SearchQuery::Move(move_pattern)
                        if has_positional
                            && !matching_plies_set.is_empty()
                            && !move_pattern.is_legal
                            && move_pattern.count_predicate.is_none() =>
                    {
                        let mut departure_matches = Vec::new();
                        for &pos_ply in &matching_plies_set {
                            if pos_ply < moves.len() {
                                let record = &moves[pos_ply];
                                if PathMatcher::match_move(
                                    move_pattern,
                                    &positions[pos_ply],
                                    record,
                                ) {
                                    departure_matches.push(pos_ply);
                                }
                            }
                        }
                        let is_match = !departure_matches.is_empty();
                        QueryMatchResult {
                            is_match,
                            match_count: departure_matches.len(),
                            matching_plies: departure_matches,
                        }
                    }
                    SearchQuery::Path(path_pattern)
                        if has_positional && !matching_plies_set.is_empty() =>
                    {
                        let mut anchored_matches = Vec::new();
                        for &start_ply in &matching_plies_set {
                            let mut anchored_pat = path_pattern.clone();
                            anchored_pat.start_ply_range = Some(start_ply..start_ply + 1);
                            if PathMatcher::match_path(&anchored_pat, positions, moves).is_some() {
                                anchored_matches.push(start_ply);
                            }
                        }
                        let is_match = !anchored_matches.is_empty();
                        QueryMatchResult {
                            is_match,
                            match_count: anchored_matches.len(),
                            matching_plies: anchored_matches,
                        }
                    }
                    _ => evaluate_with_timeline_env(q, headers, positions, moves, env),
                };

                if !res.is_match {
                    return QueryMatchResult::default();
                }
                if q.is_header_only() {
                    continue;
                }
                has_positional = true;
                if is_first_positional {
                    matching_plies_set = res.matching_plies;
                    is_first_positional = false;
                } else {
                    matching_plies_set.retain(|p| res.matching_plies.contains(p));
                    if matching_plies_set.is_empty() {
                        return QueryMatchResult::default();
                    }
                }
            }

            let final_plies = if has_positional {
                matching_plies_set
            } else {
                vec![0]
            };

            QueryMatchResult {
                is_match: true,
                match_count: final_plies.len(),
                matching_plies: final_plies,
            }
        }
        SearchQuery::Or(sub_queries) => {
            let mut any_match = false;
            let mut all_plies = Vec::new();
            for q in sub_queries {
                let res = evaluate_with_timeline_env(q, headers, positions, moves, env);
                if res.is_match {
                    any_match = true;
                    all_plies.extend(res.matching_plies);
                }
            }
            all_plies.sort_unstable();
            all_plies.dedup();
            QueryMatchResult {
                is_match: any_match,
                match_count: all_plies.len(),
                matching_plies: all_plies,
            }
        }
        SearchQuery::Annotation(ann_pred) => {
            let mut matched_plies = Vec::new();
            match ann_pred {
                crate::search::annotation::AnnotationPredicate::Comment(comment_pred) => {
                    for record in moves {
                        if let Some(ref c) = record.comment {
                            if comment_pred.matches(c) {
                                matched_plies.push(record.ply);
                            }
                        }
                    }
                }
                crate::search::annotation::AnnotationPredicate::Nag(nag_pred) => {
                    for record in moves {
                        if nag_pred.matches(&record.nags) {
                            matched_plies.push(record.ply);
                        }
                    }
                }
            }
            let is_match = !matched_plies.is_empty();
            QueryMatchResult {
                is_match,
                match_count: matched_plies.len(),
                matching_plies: matched_plies,
            }
        }
        SearchQuery::Symmetric {
            query: sub_query,
            symmetry,
        } => {
            let symmetries = symmetry.expand();
            let mut any_match = false;
            let mut all_plies = Vec::new();

            for sym in symmetries {
                let transformed = sym.transform_query(sub_query);
                let res = evaluate_with_timeline_env(&transformed, headers, positions, moves, env);
                if res.is_match {
                    any_match = true;
                    all_plies.extend(res.matching_plies);
                }
            }

            all_plies.sort_unstable();
            all_plies.dedup();
            QueryMatchResult {
                is_match: any_match,
                match_count: all_plies.len(),
                matching_plies: all_plies,
            }
        }
        SearchQuery::Not(sub_query) => {
            let res = evaluate_with_timeline_env(sub_query, headers, positions, moves, env);
            if sub_query.is_header_only() {
                let is_match = !res.is_match;
                QueryMatchResult {
                    is_match,
                    matching_plies: if is_match { vec![0] } else { Vec::new() },
                    match_count: if is_match { 1 } else { 0 },
                }
            } else {
                let non_matching_plies: Vec<usize> = (0..positions.len())
                    .filter(|p| !res.matching_plies.contains(p))
                    .collect();
                let is_match = !non_matching_plies.is_empty();
                QueryMatchResult {
                    is_match,
                    matching_plies: non_matching_plies.clone(),
                    match_count: non_matching_plies.len(),
                }
            }
        }
    }
}

pub fn quick_check_headers_only(
    query: &SearchQuery,
    headers: &HashMap<String, String>,
) -> Option<bool> {
    match query {
        SearchQuery::Header(pred) => Some(HeaderMatcher::matches(pred, headers)),
        SearchQuery::And(sub_queries) => {
            let mut all_true = true;
            for q in sub_queries {
                match quick_check_headers_only(q, headers) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => all_true = false,
                }
            }
            if all_true && !sub_queries.is_empty() {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn quick_check_entry_headers(
    query: &SearchQuery,
    entry: &chess_scid_rw::entry::IndexEntry,
    names: &chess_scid_rw::names::NameTables,
) -> Option<bool> {
    match query {
        SearchQuery::Header(pred) => HeaderMatcher::matches_entry(pred, entry, names),
        SearchQuery::And(sub_queries) => {
            let mut all_true = true;
            for q in sub_queries {
                match quick_check_entry_headers(q, entry, names) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => all_true = false,
                }
            }
            if all_true && !sub_queries.is_empty() {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn quick_check_pgn_entry_headers(
    query: &SearchQuery,
    entry: &crate::pgn_db::CompactPgnRecord,
    names: &crate::pgn_db::PgnNameTables,
) -> Option<bool> {
    match query {
        SearchQuery::Header(pred) => HeaderMatcher::matches_pgn_entry(pred, entry, names),
        SearchQuery::And(sub_queries) => {
            let mut all_true = true;
            for q in sub_queries {
                match quick_check_pgn_entry_headers(q, entry, names) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => all_true = false,
                }
            }
            if all_true && !sub_queries.is_empty() {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn matches_single_ply(
    query: &SearchQuery,
    pos: &Chess,
    ply: usize,
    last_move: Option<(&Chess, &MoveRecord)>,
) -> bool {
    match query {
        SearchQuery::Position(pattern) => {
            crate::search::pattern::PositionMatcher::matches_at_ply(pattern, pos, ply)
        }
        SearchQuery::Pawn(pred) => crate::search::pawn::PawnEvaluator::matches(pred, pos.board()),
        SearchQuery::Tactical(pred) => crate::search::tactics::TacticsEvaluator::matches(pred, pos),
        SearchQuery::Material(pred) => {
            crate::search::pattern::PositionMatcher::matches_material(pred, pos)
        }
        SearchQuery::Power(pred) => {
            crate::search::pattern::PositionMatcher::matches_power(pred, pos)
        }
        SearchQuery::Move(move_pattern) => {
            if let Some((pos_before, record)) = last_move {
                if !move_pattern.is_legal
                    && move_pattern.count_predicate.is_none()
                    && move_pattern.san.is_none()
                {
                    crate::search::path::PathMatcher::match_move(move_pattern, pos_before, record)
                } else {
                    false
                }
            } else {
                false
            }
        }
        SearchQuery::PlyRange { range, query: sub } => {
            if ply >= range.start && ply < range.end {
                matches_single_ply(sub, pos, ply, last_move)
            } else {
                false
            }
        }
        SearchQuery::Or(sub_queries) => sub_queries
            .iter()
            .any(|q| matches_single_ply(q, pos, ply, last_move)),
        SearchQuery::And(sub_queries) => {
            // Cheap predicates (squares, counts, turns) fail fast in ~1ns before expensive checkmate/tactics (~100ns)
            let mut ordered: Vec<&SearchQuery> = sub_queries.iter().collect();
            ordered.sort_by_key(|q| predicate_eval_cost(q));
            ordered
                .into_iter()
                .all(|q| matches_single_ply(q, pos, ply, last_move))
        }
        _ => false,
    }
}

#[inline]
fn predicate_eval_cost(q: &SearchQuery) -> u8 {
    match q {
        SearchQuery::Header(_) => 0,
        SearchQuery::Position(crate::search::query::PositionPattern::Squares(_)) => 1,
        SearchQuery::Position(crate::search::query::PositionPattern::PiecePlacement(_)) => 1,
        SearchQuery::Position(crate::search::query::PositionPattern::PieceCount { .. }) => 1,
        SearchQuery::Position(crate::search::query::PositionPattern::Turn(_)) => 1,
        SearchQuery::Position(crate::search::query::PositionPattern::Ply { .. }) => 1,
        SearchQuery::Position(crate::search::query::PositionPattern::MoveNumber { .. }) => 1,
        SearchQuery::Material(_) => 1,
        SearchQuery::Power(_) => 1,
        SearchQuery::Pawn(_) => 2,
        SearchQuery::Position(crate::search::query::PositionPattern::Attack { .. }) => 3,
        SearchQuery::Position(crate::search::query::PositionPattern::IsAttacked { .. }) => 3,
        SearchQuery::Position(crate::search::query::PositionPattern::BoardState {
            is_checkmate: None,
            is_stalemate: None,
            ..
        }) => 3,
        SearchQuery::Move(_) => 4,
        SearchQuery::Tactical(_) => 5,
        SearchQuery::Position(crate::search::query::PositionPattern::BoardState { .. }) => 6,
        _ => 4,
    }
}
