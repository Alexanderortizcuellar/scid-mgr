pub mod matcher;
pub mod replayer;

use shakmaty::fen::Fen;
use shakmaty::Chess;
use std::collections::HashMap;
use std::str::FromStr;

use crate::search::path::MoveRecord;
use crate::search::query::SearchQuery;

pub use matcher::{
    evaluate_with_timeline_env, matches_single_ply, quick_check_entry_headers,
    quick_check_headers_only, quick_check_pgn_entry_headers,
};
pub use replayer::{
    evaluate_pgn_streaming, parse_pgn_headers_and_moves, replay_game, split_pgn_headers_and_moves,
};

/// Match report for a game evaluation
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryMatchResult {
    pub is_match: bool,
    pub matching_plies: Vec<usize>,
    pub match_count: usize,
}

/// Search Evaluator executes queries across game positions, moves, and headers
pub struct GameSearchEvaluator;

impl GameSearchEvaluator {
    /// Evaluate a SearchQuery on a PGN string
    pub fn evaluate_pgn(query: &SearchQuery, pgn_str: &str) -> QueryMatchResult {
        let (headers, moves_str) = split_pgn_headers_and_moves(pgn_str);
        if let Some(false) = quick_check_headers_only(query, &headers) {
            return QueryMatchResult::default();
        }
        if let SearchQuery::Header(pred) = query {
            let is_match = crate::search::header::HeaderMatcher::matches(pred, &headers);
            return QueryMatchResult {
                is_match,
                matching_plies: if is_match { vec![0] } else { Vec::new() },
                match_count: if is_match { 1 } else { 0 },
            };
        }

        let start_pos = if let Some(fen_str) = headers.get("FEN") {
            Fen::from_str(fen_str.trim())
                .ok()
                .and_then(|f| {
                    f.into_position::<Chess>(shakmaty::CastlingMode::Standard)
                        .ok()
                })
                .unwrap_or_default()
        } else {
            Chess::default()
        };

        if query.can_stream_early_exit() {
            return evaluate_pgn_streaming(query, start_pos, moves_str);
        }

        let (positions, move_records) = replay_game(&headers, moves_str);
        Self::evaluate_with_timeline(query, &headers, &positions, &move_records)
    }

    /// Detailed evaluation returning match result, headers, and first matching position FEN
    pub fn evaluate_pgn_detailed(
        query: &SearchQuery,
        pgn_str: &str,
    ) -> (QueryMatchResult, HashMap<String, String>, Option<String>) {
        let (headers, moves_str) = split_pgn_headers_and_moves(pgn_str);
        if let Some(false) = quick_check_headers_only(query, &headers) {
            return (QueryMatchResult::default(), headers, None);
        }
        let (positions, move_records) = replay_game(&headers, moves_str);
        let match_result = Self::evaluate_with_timeline(query, &headers, &positions, &move_records);
        let first_fen = if match_result.is_match {
            let ply = match_result.matching_plies.first().copied().unwrap_or(0);
            positions
                .get(ply)
                .map(|p| Fen::from_position(p.clone(), shakmaty::EnPassantMode::Legal).to_string())
        } else {
            None
        };
        (match_result, headers, first_fen)
    }

    /// Evaluate a SearchQuery on parsed headers and move string
    pub fn evaluate_game(
        query: &SearchQuery,
        headers: &HashMap<String, String>,
        moves_str: &str,
    ) -> QueryMatchResult {
        // Fast-path: If root query has only header requirements and they fail, short-circuit
        if let Some(false) = quick_check_headers_only(query, headers) {
            return QueryMatchResult::default();
        }

        if query.can_stream_early_exit() {
            let start_pos = if let Some(fen_str) = headers.get("FEN") {
                Fen::from_str(fen_str.trim())
                    .ok()
                    .and_then(|f| {
                        f.into_position::<Chess>(shakmaty::CastlingMode::Standard)
                            .ok()
                    })
                    .unwrap_or_default()
            } else {
                Chess::default()
            };
            return evaluate_pgn_streaming(query, start_pos, moves_str);
        }

        // Replay game positions & collect move history
        let (positions, move_records) = replay_game(headers, moves_str);
        Self::evaluate_with_timeline(query, headers, &positions, &move_records)
    }

    /// Evaluate a SearchQuery on pre-replayed positions and move records
    pub fn evaluate_with_timeline(
        query: &SearchQuery,
        headers: &HashMap<String, String>,
        positions: &[Chess],
        moves: &[MoveRecord],
    ) -> QueryMatchResult {
        Self::evaluate_with_timeline_env(query, headers, positions, moves, &HashMap::new())
    }

    /// Evaluate a SearchQuery on pre-replayed positions and move records with variable environment
    pub fn evaluate_with_timeline_env(
        query: &SearchQuery,
        headers: &HashMap<String, String>,
        positions: &[Chess],
        moves: &[MoveRecord],
        env: &HashMap<String, shakmaty::Square>,
    ) -> QueryMatchResult {
        matcher::evaluate_with_timeline_env(query, headers, positions, moves, env)
    }
}
