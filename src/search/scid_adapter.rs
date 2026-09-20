use chess_scid_rw::entry::IndexEntry;
use chess_scid_rw::names::NameTables;
use rayon::prelude::*;
use shakmaty::{Chess, Color, Position};
use std::collections::HashMap;

use super::evaluator::{GameSearchEvaluator, QueryMatchResult};
use super::header::HeaderMatcher;
use super::path::MoveRecord;
use super::query::SearchQuery;

/// Result record for a matched game in a database
#[derive(Debug, Clone, PartialEq)]
pub struct ScidMatchResult {
    pub game_id: usize,
    pub match_details: QueryMatchResult,
}

/// Native zero-copy adapter for executing search queries on SCID binary databases
pub struct ScidSearchAdapter;

impl ScidSearchAdapter {
    /// Evaluate a SearchQuery on a SCID binary game record without PGN string allocations
    pub fn evaluate_scid_game(
        query: &SearchQuery,
        entry: &IndexEntry,
        names: &NameTables,
        blob: &[u8],
    ) -> QueryMatchResult {
        // Fast in-memory check without header_map allocation
        if let Some(false) =
            crate::search::evaluator::quick_check_entry_headers(query, entry, names)
        {
            return QueryMatchResult::default();
        }

        let has_headers = query.has_header_predicates();
        let mut header_map = HashMap::new();

        if has_headers {
            if let SearchQuery::Header(ref pred) = query {
                if let Some(is_match) = HeaderMatcher::matches_entry(pred, entry, names) {
                    return QueryMatchResult {
                        is_match,
                        matching_plies: if is_match { vec![0] } else { Vec::new() },
                        match_count: if is_match { 1 } else { 0 },
                    };
                }
            }

            header_map.insert(
                "White".to_string(),
                names.player(entry.white_id).to_string(),
            );
            header_map.insert(
                "Black".to_string(),
                names.player(entry.black_id).to_string(),
            );
            header_map.insert("Event".to_string(), names.event(entry.event_id).to_string());
            header_map.insert("Site".to_string(), names.site(entry.site_id).to_string());
            header_map.insert("Round".to_string(), names.round(entry.round_id).to_string());
            header_map.insert(
                "Date".to_string(),
                chess_scid_rw::dates::date_to_pgn(entry.date),
            );
            header_map.insert(
                "Result".to_string(),
                crate::db::result_code_to_str(entry.result).to_string(),
            );

            if let Some(eco) = chess_scid_rw::eco::eco_to_string(entry.eco_code) {
                header_map.insert("ECO".to_string(), eco);
            }
            if entry.white_elo > 0 {
                header_map.insert("WhiteElo".to_string(), entry.white_elo.to_string());
            }
            if entry.black_elo > 0 {
                header_map.insert("BlackElo".to_string(), entry.black_elo.to_string());
            }

            // Decode any custom extra tags from SCID blob
            let mut tag_cursor = 0;
            crate::position_search::decode_extra_tags(blob, &mut tag_cursor, &mut header_map);

            // Fast short-circuit if query only matches headers
            if let SearchQuery::Header(ref pred) = query {
                let is_match = HeaderMatcher::matches(pred, &header_map);
                return QueryMatchResult {
                    is_match,
                    matching_plies: if is_match { vec![0] } else { Vec::new() },
                    match_count: if is_match { 1 } else { 0 },
                };
            }

            // Short-circuit if top-level header predicates fail
            if let Some(false) =
                crate::search::evaluator::quick_check_headers_only(query, &header_map)
            {
                return QueryMatchResult::default();
            }
        }

        if entry.non_standard_start {
            if let Ok(pgn) = chess_scid_rw::pgn_build::build_pgn(entry, names, blob) {
                return GameSearchEvaluator::evaluate_pgn(query, &pgn);
            }
        }

        // Fast-path: Single-pass streaming evaluation with early exit for positional queries
        if query.can_stream_early_exit() {
            return Self::evaluate_scid_game_streaming(query, blob);
        }

        // Step 2: Stream binary moves from blob without unnecessary SAN string formatting
        let generate_san = query.requires_san_strings();
        let (positions, move_records) = Self::replay_scid_blob_opt(blob, generate_san);

        // Step 3: Evaluate full timeline
        GameSearchEvaluator::evaluate_with_timeline(query, &header_map, &positions, &move_records)
    }

    /// Single-pass streaming evaluation directly on SCID blob with zero position vector allocations and early exit
    pub fn evaluate_scid_game_streaming(query: &SearchQuery, blob: &[u8]) -> QueryMatchResult {
        let mut cursor = 0;
        let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
            Some(p) => p,
            None => {
                let default_pos = Chess::default();
                if matches_single_ply(query, &default_pos, 0, None) {
                    return QueryMatchResult {
                        is_match: true,
                        matching_plies: vec![0],
                        match_count: 1,
                    };
                }
                return QueryMatchResult::default();
            }
        };

        if cursor > 0 {
            let (positions, move_records) = Self::replay_scid_blob(blob);
            let empty_headers = std::collections::HashMap::new();
            return crate::search::evaluator::evaluate_with_timeline_env(
                query,
                &empty_headers,
                &positions,
                &move_records,
                &std::collections::HashMap::new(),
            );
        }

        let mut matching_plies = Vec::new();

        if matches_single_ply(query, &pos, 0, None) {
            matching_plies.push(0);
        }

        let max_cutoff = query.max_ply_cutoff();
        let mut slots = crate::position_search::standard_piece_slots();
        let mut counts = [16usize, 16];
        let mut ply_count = 0;

        while cursor < blob.len() {
            if let Some(cutoff) = max_cutoff {
                if ply_count >= cutoff {
                    break;
                }
            }

            let byte = blob[cursor];
            cursor += 1;
            if byte == 15 {
                break;
            }
            if byte == 11 {
                cursor += 1;
                continue;
            }
            if byte == 12 || byte == 13 || byte == 14 {
                continue;
            }

            let side_idx = usize::from(pos.turn() == Color::Black);
            let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
                match crate::position_search::decode_raw_move(
                    byte,
                    &mut cursor,
                    blob,
                    &pos,
                    &slots,
                    &counts,
                ) {
                    Some(m) => m,
                    None => break,
                };

            crate::position_search::update_slots_on_move(
                &mut slots,
                &mut counts,
                side_idx,
                piece_idx,
                to_sq,
                is_k,
                is_q,
                cap_sq,
            );

            let pos_before = pos.clone();
            pos.play_unchecked(&mv);
            ply_count += 1;
            let is_check = pos.is_check();

            let move_rec = MoveRecord {
                ply: ply_count,
                mv,
                san: String::new(),
                is_check,
                nags: Vec::new(),
                comment: None,
            };

            if matches_single_ply(query, &pos, ply_count, Some((&pos_before, &move_rec))) {
                matching_plies.push(ply_count);
            }
        }

        let is_match = !matching_plies.is_empty();
        QueryMatchResult {
            is_match,
            match_count: matching_plies.len(),
            matching_plies,
        }
    }

    /// Replay positions and moves directly from SCID binary blob
    pub fn replay_scid_blob(blob: &[u8]) -> (Vec<Chess>, Vec<MoveRecord>) {
        Self::replay_scid_blob_opt(blob, true)
    }

    /// Replay positions and moves directly from SCID binary blob with configurable SAN formatting
    pub fn replay_scid_blob_opt(blob: &[u8], generate_san: bool) -> (Vec<Chess>, Vec<MoveRecord>) {
        let mut positions = Vec::new();
        let mut move_records = Vec::new();

        let mut cursor = 0;
        let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
            Some(p) => p,
            None => {
                let default_pos = Chess::default();
                positions.push(default_pos);
                return (positions, move_records);
            }
        };

        positions.push(pos.clone());

        let mut slots = crate::position_search::standard_piece_slots();
        let mut counts = [16usize, 16];
        let mut ply_count = 0;

        while cursor < blob.len() {
            let byte = blob[cursor];
            cursor += 1;
            if byte == 15 {
                break;
            }
            if byte == 11 {
                cursor += 1;
                continue;
            }
            if byte == 12 || byte == 13 || byte == 14 {
                continue;
            }

            let side_idx = usize::from(pos.turn() == Color::Black);
            let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
                match crate::position_search::decode_raw_move(
                    byte,
                    &mut cursor,
                    blob,
                    &pos,
                    &slots,
                    &counts,
                ) {
                    Some(m) => m,
                    None => break,
                };

            crate::position_search::update_slots_on_move(
                &mut slots,
                &mut counts,
                side_idx,
                piece_idx,
                to_sq,
                is_k,
                is_q,
                cap_sq,
            );

            let san_str = if generate_san {
                shakmaty::san::SanPlus::from_move_and_play_unchecked(&mut pos, &mv).to_string()
            } else {
                pos.play_unchecked(&mv);
                String::new()
            };
            ply_count += 1;
            let is_check = pos.is_check();

            move_records.push(MoveRecord {
                ply: ply_count,
                mv,
                san: san_str,
                is_check,
                nags: Vec::new(),
                comment: None,
            });

            positions.push(pos.clone());
        }

        (positions, move_records)
    }

    /// Multi-threaded parallel search over a sub-range of SCID games [start_game..end_game]
    pub fn search_parallel_range_with_progress<'b, F, P>(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        start_game: usize,
        end_game: usize,
        get_blob: F,
        progress: P,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(&IndexEntry) -> Option<&'b [u8]> + Sync + Send,
        P: Fn(usize, usize, usize) + Sync,
    {
        let total_entries = entries.len();
        let start = start_game.min(total_entries);
        let end = end_game.min(total_entries);
        if start >= end {
            return Vec::new();
        }

        let slice = &entries[start..end];
        let total = slice.len();
        let scanned = std::sync::atomic::AtomicUsize::new(0);
        let matches_count = std::sync::atomic::AtomicUsize::new(0);
        let chunk_size = 256;

        let results: Vec<ScidMatchResult> = slice
            .par_chunks(chunk_size)
            .enumerate()
            .flat_map(|(chunk_idx, chunk)| {
                let mut local_results = Vec::new();
                let mut local_matches = 0;

                for (offset, entry) in chunk.iter().enumerate() {
                    let game_id = start + chunk_idx * chunk_size + offset;

                    if entry.deleted {
                        continue;
                    }

                    // Fast in-memory header pre-filter before reading blob from disk
                    if let Some(false) =
                        crate::search::evaluator::quick_check_entry_headers(query, entry, names)
                    {
                        continue;
                    }

                    let blob = match get_blob(entry) {
                        Some(b) => b,
                        None => continue,
                    };

                    let res = Self::evaluate_scid_game(query, entry, names, blob);
                    if res.is_match {
                        local_matches += 1;
                        local_results.push(ScidMatchResult {
                            game_id,
                            match_details: res,
                        });
                    }
                }

                let cur_scanned = scanned
                    .fetch_add(chunk.len(), std::sync::atomic::Ordering::Relaxed)
                    + chunk.len();
                let cur_matches = if local_matches > 0 {
                    matches_count.fetch_add(local_matches, std::sync::atomic::Ordering::Relaxed)
                        + local_matches
                } else {
                    matches_count.load(std::sync::atomic::Ordering::Relaxed)
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

    /// Multi-threaded parallel search over a collection of SCID games with progress streaming
    pub fn search_parallel_with_progress<'b, F, P>(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        get_blob: F,
        progress: P,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(&IndexEntry) -> Option<&'b [u8]> + Sync + Send,
        P: Fn(usize, usize, usize) + Sync,
    {
        Self::search_parallel_range_with_progress(
            query,
            entries,
            names,
            0,
            entries.len(),
            get_blob,
            progress,
        )
    }

    /// Multi-threaded parallel search over a collection of SCID games
    pub fn search_parallel<'b, F>(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        get_blob: F,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(&IndexEntry) -> Option<&'b [u8]> + Sync + Send,
    {
        Self::search_parallel_with_progress(query, entries, names, get_blob, |_, _, _| {})
    }

    /// Multi-threaded parallel search over a sub-range of SCID games [start_game..end_game]
    pub fn search_parallel_range<'b, F>(
        query: &SearchQuery,
        entries: &[IndexEntry],
        names: &NameTables,
        start_game: usize,
        end_game: usize,
        get_blob: F,
    ) -> Vec<ScidMatchResult>
    where
        F: Fn(&IndexEntry) -> Option<&'b [u8]> + Sync + Send,
    {
        Self::search_parallel_range_with_progress(
            query,
            entries,
            names,
            start_game,
            end_game,
            get_blob,
            |_, _, _| {},
        )
    }
}

fn matches_single_ply(
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
        _ => false,
    }
}
