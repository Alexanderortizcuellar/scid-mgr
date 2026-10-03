use cql_lang::evaluator::QueryMatchResult;
use cql_lang::query::{
    ComparisonOp, HeaderPredicate, MaterialPredicate, MovePattern, PositionPattern, PowerPredicate,
    SearchQuery, SquareContent,
};
use rayon::prelude::*;
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess, Color, FromSetup, Piece, Position, Role, Square};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::pgn_db::{CompactPgnRecord, PgnNameTables};
use crate::search_adapter::{
    quick_check_entry_headers, quick_check_pgn_entry_headers, ScidMatchResult, ScidSearchAdapter,
};
use crate::search_booster::codec::MmapBoostIndex;
use crate::search_booster::evaluator::{chess_to_board_array, FastReplayState};
use crate::search_booster::types::BoostMove;
use chess_scid_rw::entry::IndexEntry;
use chess_scid_rw::names::NameTables;

/// High-speed booster adapter for executing CQL search queries directly on `.boost.idx`
pub struct BoosterLanguageSearchAdapter;

impl BoosterLanguageSearchAdapter {
    /// Determines whether a SearchQuery AST can be accelerated using the booster.
    /// Returns `true` if all components can be evaluated on the booster stream (with custom FEN fallback).
    /// Returns `false` if the query requires full PGN/blob evaluation (e.g. annotations/comments or speculative mutations).
    pub fn can_booster_evaluate(query: &SearchQuery) -> bool {
        match query {
            SearchQuery::Header(_) => true,
            SearchQuery::Position(pattern) => match pattern {
                PositionPattern::BoardState {
                    is_checkmate: Some(true),
                    ..
                }
                | PositionPattern::BoardState {
                    is_stalemate: Some(true),
                    ..
                } => false,
                _ => true,
            },
            SearchQuery::Material(_)
            | SearchQuery::Power(_)
            | SearchQuery::Pawn(_)
            | SearchQuery::SquareSet(_)
            | SearchQuery::Tactical(_) => true,
            SearchQuery::Move(m) => {
                // If legal move count is queried, we cannot answer purely from played moves
                !(m.is_legal && m.count_predicate.is_some())
            }
            SearchQuery::Path(_) | SearchQuery::CqlPath(_) | SearchQuery::CqlLine(_) => false,
            SearchQuery::Annotation(_) => false,
            SearchQuery::WhatIf { .. } => false,
            SearchQuery::Play { .. } => false,
            SearchQuery::VariableBinding { query: sub, .. } => Self::can_booster_evaluate(sub),
            SearchQuery::Symmetric { query: sub, .. }
            | SearchQuery::Shift { query: sub, .. }
            | SearchQuery::PlyRange { query: sub, .. }
            | SearchQuery::Occurrences { query: sub, .. }
            | SearchQuery::Parent(sub)
            | SearchQuery::Child(sub)
            | SearchQuery::Initial(sub)
            | SearchQuery::Terminal(sub)
            | SearchQuery::Not(sub) => Self::can_booster_evaluate(sub),
            SearchQuery::And(subs) | SearchQuery::Or(subs) => {
                subs.iter().all(Self::can_booster_evaluate)
            }
        }
    }

    /// Evaluates a query across a SCID database utilizing the booster index where applicable,
    /// transparently falling back to blob parsing only for non-standard `is_custom_fen` games.
    pub fn search_scid_with_booster<F, B, BRef>(
        query: &SearchQuery,
        boost_idx: &MmapBoostIndex,
        entries: &[IndexEntry],
        names: &NameTables,
        start_game: usize,
        end_game: usize,
        get_blob: B,
        progress: F,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
        B: Fn(&IndexEntry) -> Option<BRef> + Sync,
        BRef: AsRef<[u8]>,
    {
        let end_game = end_game.min(entries.len()).min(boost_idx.game_count());
        if start_game >= end_game {
            return Vec::new();
        }

        let slice = &entries[start_game..end_game];
        let total = slice.len();
        let scanned = AtomicUsize::new(0);
        let matched = AtomicUsize::new(0);

        let results: Vec<ScidMatchResult> = slice
            .par_iter()
            .enumerate()
            .filter_map(|(relative_idx, entry)| {
                let s = scanned.fetch_add(1, Ordering::Relaxed) + 1;
                if s.is_multiple_of(5000) || s == total {
                    progress(s, total, matched.load(Ordering::Relaxed));
                }

                if entry.deleted {
                    return None;
                }

                // 1. Fast in-memory header pre-filter (< 1 ns)
                if let Some(false) = quick_check_entry_headers(query, entry, names) {
                    return None;
                }

                // 2. Pure header query fast-path
                if query.is_header_only() {
                    if let Some(true) = quick_check_entry_headers(query, entry, names) {
                        matched.fetch_add(1, Ordering::Relaxed);
                        return Some(ScidMatchResult {
                            game_id: start_game + relative_idx,
                            match_details: QueryMatchResult {
                                is_match: true,
                                matching_plies: vec![0],
                                match_count: 1,
                            },
                        });
                    }
                }

                let game_id = start_game + relative_idx;
                let boost_entry = boost_idx.get_game_entry(game_id);

                // 3. Fallback for custom start positions (Chess960 / puzzles / studies)
                if boost_entry.map(|e| e.is_custom_fen()).unwrap_or(true) {
                    let blob_opt = get_blob(entry);
                    let blob = blob_opt.as_ref().map(|b| b.as_ref()).unwrap_or(&[]);
                    let match_details =
                        ScidSearchAdapter::evaluate_scid_game(query, entry, names, blob);
                    if match_details.is_match {
                        matched.fetch_add(1, Ordering::Relaxed);
                        return Some(ScidMatchResult {
                            game_id,
                            match_details,
                        });
                    }
                    return None;
                }

                // 4. Ultra-fast Booster evaluation (~5 ns / move)
                let moves = boost_idx.get_game_moves(game_id).unwrap_or(&[]);
                let game_result = boost_entry.map(|e| e.result).unwrap_or(entry.result);
                let match_details = Self::evaluate_booster_game(query, moves, game_result, None);

                if match_details.is_match {
                    matched.fetch_add(1, Ordering::Relaxed);
                    Some(ScidMatchResult {
                        game_id,
                        match_details,
                    })
                } else {
                    None
                }
            })
            .collect();

        progress(total, total, results.len());
        results
    }

    /// Evaluates a query across a PGN database utilizing the booster index where applicable,
    /// transparently falling back to raw PGN parsing only for non-standard `is_custom_fen` games.
    pub fn search_pgn_with_booster<F, G>(
        query: &SearchQuery,
        boost_idx: &MmapBoostIndex,
        entries: &[CompactPgnRecord],
        names: &PgnNameTables,
        start_game: usize,
        end_game: usize,
        get_pgn_text: G,
        progress: F,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
        G: Fn(usize) -> anyhow::Result<String> + Sync,
    {
        let total_entries = entries.len().min(boost_idx.game_count());
        let start = start_game.min(total_entries);
        let end = end_game.min(total_entries);
        if start >= end {
            return Vec::new();
        }

        let slice = &entries[start..end];
        let total = slice.len();
        let scanned = AtomicUsize::new(0);
        let matches_count = AtomicUsize::new(0);
        let chunk_size = 256;

        let results: Vec<ScidMatchResult> = slice
            .par_chunks(chunk_size)
            .enumerate()
            .flat_map(|(chunk_idx, chunk)| {
                let mut local_results = Vec::new();
                let mut local_matches = 0;

                for (offset, entry) in chunk.iter().enumerate() {
                    let game_id = start + chunk_idx * chunk_size + offset;

                    // 1. Fast in-memory header pre-filter
                    if let Some(false) = quick_check_pgn_entry_headers(query, entry, names) {
                        continue;
                    }

                    // 2. Pure header query fast-path
                    if query.is_header_only() {
                        if let Some(true) = quick_check_pgn_entry_headers(query, entry, names) {
                            local_matches += 1;
                            local_results.push(ScidMatchResult {
                                game_id,
                                match_details: QueryMatchResult {
                                    is_match: true,
                                    matching_plies: vec![0],
                                    match_count: 1,
                                },
                            });
                            continue;
                        }
                    }

                    let boost_entry = boost_idx.get_game_entry(game_id);

                    // 3. Fallback for custom start positions
                    if boost_entry.map(|e| e.is_custom_fen()).unwrap_or(true) {
                        if let Ok(pgn_text) = get_pgn_text(game_id) {
                            let res = cql_lang::evaluator::GameSearchEvaluator::evaluate_pgn(
                                query, &pgn_text,
                            );
                            if res.is_match {
                                local_matches += 1;
                                local_results.push(ScidMatchResult {
                                    game_id,
                                    match_details: res,
                                });
                            }
                        }
                        continue;
                    }

                    // 4. Ultra-fast Booster evaluation
                    let moves = boost_idx.get_game_moves(game_id).unwrap_or(&[]);
                    let game_result = boost_entry.map(|e| e.result).unwrap_or(0);
                    let res = Self::evaluate_booster_game(query, moves, game_result, None);

                    if res.is_match {
                        local_matches += 1;
                        local_results.push(ScidMatchResult {
                            game_id,
                            match_details: res,
                        });
                    }
                }

                let cur_scanned = scanned.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
                let cur_matches = if local_matches > 0 {
                    matches_count.fetch_add(local_matches, Ordering::Relaxed) + local_matches
                } else {
                    matches_count.load(Ordering::Relaxed)
                };

                if cur_scanned % 4096 < chunk.len() || cur_scanned >= total {
                    progress(cur_scanned.min(total), total, cur_matches);
                }

                local_results
            })
            .collect();

        progress(total, total, results.len());
        results
    }

    /// Evaluates a SearchQuery directly over a sequence of packed `BoostMove` half-moves
    pub fn evaluate_booster_game(
        query: &SearchQuery,
        moves: &[BoostMove],
        game_result: u8,
        max_ply: Option<usize>,
    ) -> QueryMatchResult {
        let max_cutoff = query.max_ply_cutoff().unwrap_or(usize::MAX);
        let limit = match max_ply {
            Some(mp) => moves.len().min(mp).min(max_cutoff),
            None => moves.len().min(max_cutoff),
        };

        // Replay timeline positions
        let mut replay = FastReplayState::new();
        let mut matched_plies = Vec::new();

        // Check starting position (ply 0)
        if matches_booster_ply(query, &replay, 0, None, game_result) {
            matched_plies.push(0);
        }

        // Fast early exit for simple positional queries if target material is higher than available
        let (min_w, min_b) = target_piece_counts_bound(query);

        for (ply_idx, &m) in moves[..limit].iter().enumerate() {
            let prev_replay = replay.clone();
            replay.apply_move(m);

            if replay.white_pieces < min_w || replay.black_pieces < min_b {
                break;
            }

            let ply = ply_idx + 1;
            let last_move = Some((&prev_replay, m));

            if matches_booster_ply(query, &replay, ply, last_move, game_result) {
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
}

/// Helper converting `[u8; 64]` scratchpad array into standard `shakmaty::Board`
pub fn board_array_to_shakmaty_board(board: &[u8; 64]) -> shakmaty::Board {
    let mut b = shakmaty::Board::empty();
    for (i, &val) in board.iter().enumerate() {
        if val > 0 {
            let color = if val <= 6 { Color::White } else { Color::Black };
            let role_val = if val <= 6 { val } else { val - 8 };
            let role = match role_val {
                1 => Role::Pawn,
                2 => Role::Knight,
                3 => Role::Bishop,
                4 => Role::Rook,
                5 => Role::Queen,
                6 => Role::King,
                _ => continue,
            };
            let sq = Square::new(i as u32);
            b.set_piece_at(sq, Piece { color, role });
        }
    }
    b
}

/// Helper converting `[u8; 64]` scratchpad array into standard `shakmaty::Chess`
pub fn board_array_to_shakmaty_chess(board: &[u8; 64], ply: usize) -> Option<Chess> {
    let b = board_array_to_shakmaty_board(board);
    let turn = if ply % 2 == 0 {
        Color::White
    } else {
        Color::Black
    };
    let mut setup = shakmaty::Setup::empty();
    setup.board = b;
    setup.turn = turn;
    Chess::from_setup(setup, CastlingMode::Chess960).ok()
}

/// Evaluate a single ply against `FastReplayState` and move information
pub fn matches_booster_ply(
    query: &SearchQuery,
    state: &FastReplayState,
    ply: usize,
    last_move: Option<(&FastReplayState, BoostMove)>,
    game_result: u8,
) -> bool {
    match query {
        SearchQuery::Position(pattern) => match_booster_position(pattern, state, ply),
        SearchQuery::Material(pred) => match_booster_material(pred, state),
        SearchQuery::Power(pred) => match_booster_power(pred, state),
        SearchQuery::Pawn(pred) => {
            let board = board_array_to_shakmaty_board(&state.board);
            cql_lang::pawn::PawnEvaluator::matches(pred, &board)
        }
        SearchQuery::SquareSet(pred) => {
            if let Some(pos) = board_array_to_shakmaty_chess(&state.board, ply) {
                cql_lang::squares::SquareSetEvaluator::matches(pred, &pos)
            } else {
                false
            }
        }
        SearchQuery::Tactical(pred) => {
            if let Some(pos) = board_array_to_shakmaty_chess(&state.board, ply) {
                cql_lang::tactics::TacticsEvaluator::matches(pred, &pos)
            } else {
                false
            }
        }
        SearchQuery::Move(move_pattern) => {
            if let Some((prev_state, bm)) = last_move {
                if move_pattern.is_previous {
                    match_boost_move_pattern(move_pattern, bm, prev_state, state, ply)
                } else {
                    false
                }
            } else {
                false
            }
        }
        SearchQuery::Header(HeaderPredicate::Result { expected }) => {
            let actual = match game_result {
                1 => "1-0",
                2 => "0-1",
                3 => "1/2-1/2",
                _ => "*",
            };
            if expected == "All" || (expected == "*" && actual.is_empty()) {
                true
            } else {
                actual == expected
            }
        }
        SearchQuery::Header(_) => true,
        SearchQuery::PlyRange { range, query: sub } => {
            if ply >= range.start && ply < range.end {
                matches_booster_ply(sub, state, ply, last_move, game_result)
            } else {
                false
            }
        }
        SearchQuery::Initial(sub) => {
            if ply == 0 {
                matches_booster_ply(sub, state, ply, last_move, game_result)
            } else {
                false
            }
        }
        SearchQuery::Parent(sub) => {
            if let Some((prev_state, _)) = last_move {
                if ply > 0 {
                    matches_booster_ply(sub, prev_state, ply - 1, None, game_result)
                } else {
                    false
                }
            } else {
                false
            }
        }
        SearchQuery::Symmetric {
            query: sub,
            symmetry,
        } => {
            let symmetries = symmetry.expand();
            symmetries.into_iter().any(|sym| {
                let transformed = sym.transform_query(sub);
                matches_booster_ply(&transformed, state, ply, last_move, game_result)
            })
        }
        SearchQuery::Shift { mode, query: sub } => {
            let offsets: Vec<(i8, i8)> = match mode {
                cql_lang::transform::ShiftMode::Horizontal => (-7..=7).map(|df| (df, 0)).collect(),
                cql_lang::transform::ShiftMode::Vertical => (-7..=7).map(|dr| (0, dr)).collect(),
                cql_lang::transform::ShiftMode::All => {
                    let mut v = Vec::with_capacity(15 * 15);
                    for df in -7..=7 {
                        for dr in -7..=7 {
                            v.push((df, dr));
                        }
                    }
                    v
                }
            };
            offsets.into_iter().any(|(df, dr)| {
                if let Some(shifted) = cql_lang::transform::shift_query(sub, df, dr) {
                    matches_booster_ply(&shifted, state, ply, last_move, game_result)
                } else {
                    false
                }
            })
        }
        SearchQuery::And(subs) => subs
            .iter()
            .all(|q| matches_booster_ply(q, state, ply, last_move, game_result)),
        SearchQuery::Or(subs) => subs
            .iter()
            .any(|q| matches_booster_ply(q, state, ply, last_move, game_result)),
        SearchQuery::Not(sub) => !matches_booster_ply(sub, state, ply, last_move, game_result),
        _ => false,
    }
}

/// Matches a position pattern directly against `FastReplayState`
fn match_booster_position(pattern: &PositionPattern, state: &FastReplayState, ply: usize) -> bool {
    match pattern {
        PositionPattern::Ply { op, value } => compare_usize(ply, *value, *op),
        PositionPattern::MoveNumber { op, value } => {
            let movenum = if ply == 0 { 1 } else { ply.div_ceil(2) };
            compare_usize(movenum, *value, *op)
        }
        PositionPattern::Turn(col) => {
            let expected_rem = if *col == Color::White { 0 } else { 1 };
            (ply % 2) == expected_rem
        }
        PositionPattern::Squares(square_map) => {
            for (&sq, content) in square_map {
                let piece_code = state.board[sq as usize];
                if !match_square_content_booster(piece_code, content) {
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
            let actual_count = if let Some(sqs) = squares {
                sqs.iter()
                    .filter(|&&sq| match_square_content_booster(state.board[sq as usize], content))
                    .count()
            } else {
                state
                    .board
                    .iter()
                    .filter(|&&pc| match_square_content_booster(pc, content))
                    .count()
            };
            compare_usize(actual_count, *count, *op)
        }
        PositionPattern::ExactFen(fen_str) => {
            if let Ok(fen) = fen_str.trim().parse::<Fen>() {
                if let Ok(target_pos) = fen.into_position::<Chess>(CastlingMode::Chess960) {
                    let target_board = chess_to_board_array(&target_pos);
                    let target_turn_rem = if target_pos.turn() == Color::White {
                        0
                    } else {
                        1
                    };
                    return state.board == target_board && (ply % 2) == target_turn_rem;
                }
            }
            if let Some(pos) = board_array_to_shakmaty_chess(&state.board, ply) {
                cql_lang::pattern::PositionMatcher::matches_at_ply(pattern, &pos, ply)
            } else {
                false
            }
        }
        PositionPattern::PiecePlacement(placement_str) => {
            if let Some(pos) = board_array_to_shakmaty_chess(&state.board, ply) {
                cql_lang::pattern::match_wildcard_fen(&pos, placement_str)
            } else {
                false
            }
        }
        PositionPattern::MultiSquare { content, squares } => squares
            .iter()
            .any(|&sq| match_square_content_booster(state.board[sq as usize], content)),
        _ => {
            if let Some(pos) = board_array_to_shakmaty_chess(&state.board, ply) {
                cql_lang::pattern::PositionMatcher::matches_at_ply(pattern, &pos, ply)
            } else {
                false
            }
        }
    }
}

/// Matches material composition directly against `FastReplayState`
fn match_booster_material(pred: &MaterialPredicate, state: &FastReplayState) -> bool {
    let mut w_p = 0;
    let mut w_n = 0;
    let mut w_b = 0;
    let mut w_r = 0;
    let mut w_q = 0;
    let mut w_lb = 0;
    let mut w_db = 0;

    let mut b_p = 0;
    let mut b_n = 0;
    let mut b_b = 0;
    let mut b_r = 0;
    let mut b_q = 0;
    let mut b_lb = 0;
    let mut b_db = 0;

    for (idx, &pc) in state.board.iter().enumerate() {
        let is_light = ((idx % 8) + (idx / 8)) % 2 != 0;
        match pc {
            1 => w_p += 1,
            2 => w_n += 1,
            3 => {
                w_b += 1;
                if is_light {
                    w_lb += 1;
                } else {
                    w_db += 1;
                }
            }
            4 => w_r += 1,
            5 => w_q += 1,
            9 => b_p += 1,
            10 => b_n += 1,
            11 => {
                b_b += 1;
                if is_light {
                    b_lb += 1;
                } else {
                    b_db += 1;
                }
            }
            12 => b_r += 1,
            13 => b_q += 1,
            _ => {}
        }
    }

    if let Some(v) = pred.white_pawns {
        if w_p != v {
            return false;
        }
    }
    if let Some(v) = pred.white_knights {
        if w_n != v {
            return false;
        }
    }
    if let Some(v) = pred.white_bishops {
        if w_b != v {
            return false;
        }
    }
    if let Some(v) = pred.white_rooks {
        if w_r != v {
            return false;
        }
    }
    if let Some(v) = pred.white_queens {
        if w_q != v {
            return false;
        }
    }
    if let Some(v) = pred.black_pawns {
        if b_p != v {
            return false;
        }
    }
    if let Some(v) = pred.black_knights {
        if b_n != v {
            return false;
        }
    }
    if let Some(v) = pred.black_bishops {
        if b_b != v {
            return false;
        }
    }
    if let Some(v) = pred.black_rooks {
        if b_r != v {
            return false;
        }
    }
    if let Some(v) = pred.black_queens {
        if b_q != v {
            return false;
        }
    }
    if let Some(v) = pred.white_light_bishops {
        if w_lb != v {
            return false;
        }
    }
    if let Some(v) = pred.white_dark_bishops {
        if w_db != v {
            return false;
        }
    }
    if let Some(v) = pred.black_light_bishops {
        if b_lb != v {
            return false;
        }
    }
    if let Some(v) = pred.black_dark_bishops {
        if b_db != v {
            return false;
        }
    }
    if let Some(v) = pred.light_bishops {
        if (w_lb + b_lb) != v {
            return false;
        }
    }
    if let Some(v) = pred.dark_bishops {
        if (w_db + b_db) != v {
            return false;
        }
    }

    if let Some(opp) = pred.opposite_bishops {
        let is_opp = (w_lb == 1 && w_db == 0 && b_db == 1 && b_lb == 0)
            || (w_db == 1 && w_lb == 0 && b_lb == 1 && b_db == 0);
        if is_opp != opp {
            return false;
        }
    }

    if let Some(same) = pred.same_colored_bishops {
        let is_same = (w_lb == 1 && w_db == 0 && b_lb == 1 && b_db == 0)
            || (w_dark_bishops(w_db) && w_lb == 0 && b_db == 1 && b_lb == 0);
        if is_same != same {
            return false;
        }
    }

    if let Some((op, diff)) = pred.material_difference {
        let w_pts = w_p + (w_n * 3) + (w_b * 3) + (w_r * 5) + (w_q * 9);
        let b_pts = b_p + (b_n * 3) + (b_b * 3) + (b_r * 5) + (b_q * 9);
        let actual_diff = (w_pts as i32) - (b_pts as i32);
        if !compare_i32(actual_diff, diff, op) {
            return false;
        }
    }

    true
}

#[inline(always)]
fn w_dark_bishops(w_db: usize) -> bool {
    w_db == 1
}

/// Matches piece power directly against `FastReplayState`
fn match_booster_power(pred: &PowerPredicate, state: &FastReplayState) -> bool {
    let mut w_power = 0i32;
    let mut b_power = 0i32;

    for &pc in state.board.iter() {
        match pc {
            1 => w_power += 1,
            2 | 3 => w_power += 3,
            4 => w_power += 5,
            5 => w_power += 9,
            9 => b_power += 1,
            10 | 11 => b_power += 3,
            12 => b_power += 5,
            13 => b_power += 9,
            _ => {}
        }
    }

    match pred {
        PowerPredicate::WhitePower { op, value } => compare_i32(w_power, *value, *op),
        PowerPredicate::BlackPower { op, value } => compare_i32(b_power, *value, *op),
        PowerPredicate::TotalPower { op, value } => compare_i32(w_power + b_power, *value, *op),
        PowerPredicate::WhiteVsBlackPower { op } => compare_i32(w_power, b_power, *op),
        PowerPredicate::PowerDifference {
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

/// Matches a played `BoostMove` against a `MovePattern`
fn match_boost_move_pattern(
    pat: &MovePattern,
    m: BoostMove,
    prev_state: &FastReplayState,
    _cur_state: &FastReplayState,
    ply: usize,
) -> bool {
    let from_sq = m.from();
    let to_sq = m.to();

    if let Some(f) = pat.from {
        if f as usize != from_sq {
            return false;
        }
    }
    if let Some(t) = pat.to {
        if t as usize != to_sq {
            return false;
        }
    }
    if let Some(ref fs) = pat.from_squares {
        if !fs.iter().any(|&s| s as usize == from_sq) {
            return false;
        }
    }
    if let Some(ref ts) = pat.to_squares {
        if !ts.iter().any(|&s| s as usize == to_sq) {
            return false;
        }
    }

    let moving_pc = prev_state.board[from_sq];
    if moving_pc == 0 {
        return false;
    }

    let color = if moving_pc <= 6 {
        Color::White
    } else {
        Color::Black
    };
    let role = match if moving_pc <= 6 {
        moving_pc
    } else {
        moving_pc - 8
    } {
        1 => Role::Pawn,
        2 => Role::Knight,
        3 => Role::Bishop,
        4 => Role::Rook,
        5 => Role::Queen,
        6 => Role::King,
        _ => return false,
    };

    if let Some(c) = pat.color {
        if color != c {
            return false;
        }
    }
    if let Some(r) = pat.role {
        if role != r {
            return false;
        }
    }

    if let Some(cap) = pat.is_capture {
        if m.is_capture() != cap {
            return false;
        }
    }

    if let Some(cas) = pat.is_castle {
        if m.is_castle() != cas {
            return false;
        }
    }

    if let Some(ep) = pat.is_en_passant {
        if m.is_en_passant() != ep {
            return false;
        }
    }

    if let Some(promo) = pat.promotion {
        if m.promotion_role() != Some(promo) {
            return false;
        }
    }

    if let Some(ref promos) = pat.promotions {
        if let Some(p_role) = m.promotion_role() {
            if !promos.contains(&p_role) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(ref uci_expected) = pat.uci {
        if m.to_uci_string() != *uci_expected {
            return false;
        }
    }

    if let Some(ref san_expected) = pat.san {
        if let Some(chess) = board_array_to_shakmaty_chess(&prev_state.board, ply.saturating_sub(1))
        {
            if let Some(san_str) = m.to_san_string(&chess) {
                if san_str != *san_expected {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

fn match_square_content_booster(piece_code: u8, content: &SquareContent) -> bool {
    match content {
        SquareContent::Empty => piece_code == 0,
        SquareContent::Occupied => piece_code > 0,
        SquareContent::Piece(p) => {
            if piece_code == 0 {
                return false;
            }
            let is_white = piece_code <= 6;
            let col = if is_white { Color::White } else { Color::Black };
            if p.color != col {
                return false;
            }
            let r_code = if is_white { piece_code } else { piece_code - 8 };
            let role = match r_code {
                1 => Role::Pawn,
                2 => Role::Knight,
                3 => Role::Bishop,
                4 => Role::Rook,
                5 => Role::Queen,
                6 => Role::King,
                _ => return false,
            };
            p.role == role
        }
        SquareContent::Role(r) => {
            if piece_code == 0 {
                return false;
            }
            let r_code = if piece_code <= 6 {
                piece_code
            } else {
                piece_code - 8
            };
            let role = match r_code {
                1 => Role::Pawn,
                2 => Role::Knight,
                3 => Role::Bishop,
                4 => Role::Rook,
                5 => Role::Queen,
                6 => Role::King,
                _ => return false,
            };
            *r == role
        }
        SquareContent::Color(c) => {
            if piece_code == 0 {
                return false;
            }
            let col = if piece_code <= 6 {
                Color::White
            } else {
                Color::Black
            };
            *c == col
        }
        SquareContent::AnyOf(pieces) => pieces
            .iter()
            .any(|p| match_square_content_booster(piece_code, &SquareContent::Piece(*p))),
        SquareContent::NoneOf(pieces) => !pieces
            .iter()
            .any(|p| match_square_content_booster(piece_code, &SquareContent::Piece(*p))),
    }
}

/// Helper extracting lower bound for white/black piece counts for fast branch pruning
fn target_piece_counts_bound(query: &SearchQuery) -> (u8, u8) {
    match query {
        SearchQuery::Position(PositionPattern::ExactFen(fen_str)) => {
            if let Ok(fen) = fen_str.trim().parse::<Fen>() {
                if let Ok(target_pos) = fen.into_position::<Chess>(CastlingMode::Chess960) {
                    let arr = chess_to_board_array(&target_pos);
                    return FastReplayState::count_pieces(&arr);
                }
            }
            (0, 0)
        }
        SearchQuery::Material(pred) => {
            let mut w = 0u8;
            let mut b = 0u8;
            if let Some(c) = pred.white_pawns {
                w += c as u8;
            }
            if let Some(c) = pred.white_knights {
                w += c as u8;
            }
            if let Some(c) = pred.white_bishops {
                w += c as u8;
            }
            if let Some(c) = pred.white_rooks {
                w += c as u8;
            }
            if let Some(c) = pred.white_queens {
                w += c as u8;
            }
            if let Some(c) = pred.black_pawns {
                b += c as u8;
            }
            if let Some(c) = pred.black_knights {
                b += c as u8;
            }
            if let Some(c) = pred.black_bishops {
                b += c as u8;
            }
            if let Some(c) = pred.black_rooks {
                b += c as u8;
            }
            if let Some(c) = pred.black_queens {
                b += c as u8;
            }
            (w, b)
        }
        _ => (0, 0),
    }
}

#[inline(always)]
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

#[inline(always)]
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
