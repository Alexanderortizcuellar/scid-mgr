use anyhow::Result;
use rayon::prelude::*;
use shakmaty::Position;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::core::PgnDatabaseWrapper;

// ---------------------------------------------------------------------------
// Position & Material Search Adapters for Raw PGN
// ---------------------------------------------------------------------------

impl PgnDatabaseWrapper {
    /// Search games by board position or partial piece placement across raw PGN move streams
    pub fn search_position<F>(
        &self,
        fen_str: &str,
        turn_param: Option<&str>,
        mode_param: Option<&str>,
        max_ply: Option<usize>,
        mut progress: F,
    ) -> Result<crate::position_search::PositionSearchResult>
    where
        F: FnMut(usize, usize, usize),
    {
        let start = Instant::now();
        let target_fen = fen_str.trim();
        let is_exact_mode = mode_param
            .map(|m| {
                let m = m.to_lowercase();
                m == "exact" || m == "auto" || m.is_empty()
            })
            .unwrap_or(true);

        if is_exact_mode && turn_param.is_none() {
            if let Some((_pos, zobrist_hash)) =
                crate::position_index::parse_target_position(target_fen)
            {
                if let Ok(pos_idx) = crate::position_index::PositionIndex::load(&self.pgn_path) {
                    if let Some(gids) = pos_idx.get_all_position_games(zobrist_hash) {
                        let matches: Vec<crate::position_search::PositionMatch> = gids
                            .into_iter()
                            .map(|gid| crate::position_search::PositionMatch {
                                game_id: gid,
                                ply: 0,
                            })
                            .collect();
                        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
                        progress(self.entries.len(), self.entries.len(), matches.len());
                        return Ok(crate::position_search::PositionSearchResult {
                            target_fen: target_fen.to_string(),
                            target_hash: zobrist_hash,
                            matches,
                            total_games_searched: self.entries.len(),
                            elapsed_ms,
                        });
                    }
                }
            }
        }

        let max_ply_val = max_ply.unwrap_or(500);

        let matcher =
            crate::position_search::parse_position_matcher(target_fen, turn_param, mode_param)?;

        let total = self.entries.len();
        let chunk_size = 1000;
        let mut matches = Vec::new();

        for chunk_idx in (0..total).step_by(chunk_size) {
            let end_idx = (chunk_idx + chunk_size).min(total);
            let chunk = &self.entries[chunk_idx..end_idx];

            let chunk_matches: Vec<crate::position_search::PositionMatch> = chunk
                .par_iter()
                .enumerate()
                .filter_map(|(sub_idx, entry)| {
                    let game_id = chunk_idx + sub_idx;
                    let slice = &self.mmap
                        [entry.offset as usize..(entry.offset as usize + entry.length as usize)];
                    let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                    let mut finder = PositionFinder::new(matcher.clone(), max_ply_val);
                    if let Ok(Some(Some(ply))) = reader.read_game(&mut finder) {
                        Some(crate::position_search::PositionMatch { game_id, ply })
                    } else {
                        None
                    }
                })
                .collect();

            matches.extend(chunk_matches);
            progress(end_idx, total, matches.len());
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(crate::position_search::PositionSearchResult {
            target_fen: target_fen.to_string(),
            target_hash: 0,
            matches,
            total_games_searched: total,
            elapsed_ms,
        })
    }

    /// Search games by piece count and opposite/same-colored bishops across raw PGN move streams
    pub fn search_material<F>(
        &self,
        filter: &crate::position_search::MaterialFilter,
        mut progress: F,
    ) -> Result<Vec<usize>>
    where
        F: FnMut(usize, usize, usize),
    {
        let total = self.entries.len();
        let chunk_size = 1000;
        let mut matches = Vec::new();

        for chunk_idx in (0..total).step_by(chunk_size) {
            let end_idx = (chunk_idx + chunk_size).min(total);
            let chunk = &self.entries[chunk_idx..end_idx];

            let chunk_matches: Vec<usize> = chunk
                .par_iter()
                .enumerate()
                .filter_map(|(sub_idx, entry)| {
                    let game_id = chunk_idx + sub_idx;
                    let slice = &self.mmap
                        [entry.offset as usize..(entry.offset as usize + entry.length as usize)];
                    let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                    let mut finder = MaterialFinder::new(filter.clone());
                    if let Ok(Some(true)) = reader.read_game(&mut finder) {
                        Some(game_id)
                    } else {
                        None
                    }
                })
                .collect();

            matches.extend(chunk_matches);
            progress(end_idx, total, matches.len());
        }

        Ok(matches)
    }

    /// Execute a unified SearchQuery across a sub-range of games [start_game..end_game] in the PGN database with progress streaming
    pub fn search_query_range_with_progress<F>(
        &self,
        query: &crate::search::query::SearchQuery,
        start_game: usize,
        end_game: usize,
        progress: F,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        let total_entries = self.game_count();
        let start = start_game.min(total_entries);
        let end = end_game.min(total_entries);
        if start >= end {
            return Vec::new();
        }

        let slice = &self.entries[start..end];
        let total = slice.len();
        let scanned = AtomicUsize::new(0);
        let matches_count = AtomicUsize::new(0);
        let chunk_size = 256;

        let results: Vec<crate::search::scid_adapter::ScidMatchResult> = slice
            .par_chunks(chunk_size)
            .enumerate()
            .flat_map(|(chunk_idx, chunk)| {
                let mut local_results = Vec::new();
                let mut local_matches = 0;

                for (offset, entry) in chunk.iter().enumerate() {
                    let game_id = start + chunk_idx * chunk_size + offset;

                    // Fast in-memory header pre-filter before reading PGN text from mmap
                    if let Some(false) = crate::search::evaluator::quick_check_pgn_entry_headers(
                        query,
                        entry,
                        &self.names,
                    ) {
                        continue;
                    }

                    // Fast-path: Header-only query that matched in-memory
                    if query.is_header_only() {
                        if let Some(true) = crate::search::evaluator::quick_check_pgn_entry_headers(
                            query,
                            entry,
                            &self.names,
                        ) {
                            local_matches += 1;
                            local_results.push(crate::search::scid_adapter::ScidMatchResult {
                                game_id,
                                match_details: crate::search::evaluator::QueryMatchResult {
                                    is_match: true,
                                    matching_plies: vec![0],
                                    match_count: 1,
                                },
                            });
                            continue;
                        }
                    }

                    let pgn_text = match self.get_game_pgn(game_id) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };

                    let res = crate::search::evaluator::GameSearchEvaluator::evaluate_pgn(
                        query, &pgn_text,
                    );
                    if res.is_match {
                        local_matches += 1;
                        local_results.push(crate::search::scid_adapter::ScidMatchResult {
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

    /// Execute a unified SearchQuery across the PGN database in parallel with progress streaming
    pub fn search_query_with_progress<F>(
        &self,
        query: &crate::search::query::SearchQuery,
        progress: F,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        self.search_query_range_with_progress(query, 0, self.game_count(), progress)
    }

    /// Execute a unified SearchQuery across the PGN database in parallel
    pub fn search_query(
        &self,
        query: &crate::search::query::SearchQuery,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult> {
        self.search_query_with_progress(query, |_, _, _| {})
    }

    /// Execute a unified SearchQuery across a sub-range of games [start_game..end_game] in parallel
    pub fn search_query_range(
        &self,
        query: &crate::search::query::SearchQuery,
        start_game: usize,
        end_game: usize,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult> {
        self.search_query_range_with_progress(query, start_game, end_game, |_, _, _| {})
    }
}

// ---------------------------------------------------------------------------
// PGN Visitors for Search Engine
// ---------------------------------------------------------------------------

struct PositionFinder {
    matcher: crate::position_search::PositionTargetMatcher,
    found_ply: Option<usize>,
    ply: usize,
    max_ply: usize,
    pos: shakmaty::Chess,
}

impl PositionFinder {
    fn new(matcher: crate::position_search::PositionTargetMatcher, max_ply: usize) -> Self {
        let mut s = Self {
            matcher,
            found_ply: None,
            ply: 0,
            max_ply,
            pos: shakmaty::Chess::default(),
        };
        s.check_current_pos();
        s
    }

    fn check_current_pos(&mut self) {
        if self.found_ply.is_some() {
            return;
        }
        if self.matcher.matches(&self.pos) {
            self.found_ply = Some(self.ply);
        }
    }
}

impl pgn_reader::Visitor for PositionFinder {
    type Result = Option<usize>;

    fn begin_game(&mut self) {
        self.check_current_pos();
    }

    fn begin_variation(&mut self) -> pgn_reader::Skip {
        pgn_reader::Skip(true)
    }

    fn san(&mut self, san_plus: shakmaty::san::SanPlus) {
        if self.found_ply.is_some() || self.ply >= self.max_ply {
            return;
        }

        if let Ok(m) = san_plus.san.to_move(&self.pos) {
            self.pos.play_unchecked(&m);
            self.ply += 1;
            self.check_current_pos();
        }
    }

    fn end_game(&mut self) -> Self::Result {
        self.found_ply
    }
}

struct MaterialFinder {
    filter: crate::position_search::MaterialFilter,
    matched: bool,
    ply: usize,
    max_ply: usize,
    match_any_ply: bool,
    pos: shakmaty::Chess,
}

impl MaterialFinder {
    fn new(filter: crate::position_search::MaterialFilter) -> Self {
        let max_ply = filter.max_ply.unwrap_or(500);
        let match_any_ply = filter.match_any_ply;
        let mut mf = Self {
            filter,
            matched: false,
            ply: 0,
            max_ply,
            match_any_ply,
            pos: shakmaty::Chess::default(),
        };
        if mf.match_any_ply && mf.check_material() {
            mf.matched = true;
        }
        mf
    }

    fn check_material(&self) -> bool {
        crate::position_search::matches_material(&self.pos, &self.filter)
    }
}

impl pgn_reader::Visitor for MaterialFinder {
    type Result = bool;

    fn begin_variation(&mut self) -> pgn_reader::Skip {
        pgn_reader::Skip(true)
    }

    fn san(&mut self, san_plus: shakmaty::san::SanPlus) {
        if self.matched && !self.match_any_ply {
            return;
        }
        if self.ply >= self.max_ply {
            return;
        }

        if let Ok(m) = san_plus.san.to_move(&self.pos) {
            self.pos.play_unchecked(&m);
            self.ply += 1;
            if self.match_any_ply && !self.matched && self.check_material() {
                self.matched = true;
            }
        }
    }

    fn end_game(&mut self) -> Self::Result {
        if !self.match_any_ply {
            self.check_material()
        } else {
            self.matched
        }
    }
}
