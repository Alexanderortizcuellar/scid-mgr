use anyhow::Result;

use super::core::ScidDatabaseWrapper;

impl ScidDatabaseWrapper {
    pub fn search_position_with_progress<F>(
        &self,
        fen_str: &str,
        turn_param: Option<&str>,
        mode_param: Option<&str>,
        max_ply: Option<usize>,
        progress: F,
    ) -> Result<crate::position_search::PositionSearchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        let start_time = std::time::Instant::now();
        let is_exact_mode = mode_param
            .map(|m| {
                let m = m.to_lowercase();
                m == "exact" || m == "auto" || m.is_empty()
            })
            .unwrap_or(true);

        if is_exact_mode && turn_param.is_none() {
            if let Some((_pos, zobrist_hash)) =
                crate::position_index::parse_target_position(fen_str)
            {
                if let Ok(pos_idx) = crate::position_index::PositionIndex::load(&self.index_path) {
                    if let Some(gids) = pos_idx.get_all_position_games(zobrist_hash) {
                        let matches: Vec<crate::position_search::PositionMatch> = gids
                            .into_iter()
                            .map(|gid| crate::position_search::PositionMatch {
                                game_id: gid,
                                ply: 0,
                            })
                            .collect();
                        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                        progress(self.entries.len(), self.entries.len(), matches.len());
                        return Ok(crate::position_search::PositionSearchResult {
                            target_fen: fen_str.to_string(),
                            target_hash: zobrist_hash,
                            matches,
                            total_games_searched: self.entries.len(),
                            elapsed_ms,
                        });
                    }
                }
            }
        }

        let matcher =
            crate::position_search::parse_position_matcher(fen_str, turn_param, mode_param)?;
        let matches = crate::position_search::search_position_matcher_mmap_with_progress(
            &self.entries,
            &self.games_path,
            &matcher,
            max_ply,
            progress,
        )?;
        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        Ok(crate::position_search::PositionSearchResult {
            target_fen: fen_str.to_string(),
            target_hash: 0,
            matches,
            total_games_searched: self.entries.len(),
            elapsed_ms,
        })
    }

    pub fn search_position(
        &self,
        fen_str: &str,
        turn_param: Option<&str>,
        mode_param: Option<&str>,
        max_ply: Option<usize>,
    ) -> Result<crate::position_search::PositionSearchResult> {
        self.search_position_with_progress(fen_str, turn_param, mode_param, max_ply, |_, _, _| {})
    }

    pub fn search_material_with_progress<F>(
        &self,
        filter: &crate::position_search::MaterialFilter,
        progress: F,
    ) -> Result<Vec<usize>>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        crate::position_search::search_material_mmap_with_progress(
            &self.entries,
            &self.games_path,
            filter,
            progress,
        )
    }

    pub fn search_material(
        &self,
        filter: &crate::position_search::MaterialFilter,
    ) -> Result<Vec<usize>> {
        self.search_material_with_progress(filter, |_, _, _| {})
    }

    /// Execute a unified SearchQuery across the database in parallel with progress streaming
    pub fn search_query_with_progress<F>(
        &self,
        query: &crate::search::query::SearchQuery,
        progress: F,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        crate::search::scid_adapter::ScidSearchAdapter::search_parallel_with_progress(
            query,
            self.entries(),
            self.names(),
            |entry| self.get_blob(entry).ok(),
            progress,
        )
    }

    pub(crate) fn search_query_progress_helper<F>(
        &self,
        query: &crate::search::query::SearchQuery,
        progress: &F,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult>
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        crate::search::scid_adapter::ScidSearchAdapter::search_parallel_with_progress(
            query,
            self.entries(),
            self.names(),
            |entry| self.get_blob(entry).ok(),
            progress,
        )
    }

    /// Execute a unified SearchQuery across the database in parallel
    pub fn search_query(
        &self,
        query: &crate::search::query::SearchQuery,
    ) -> Vec<crate::search::scid_adapter::ScidMatchResult> {
        self.search_query_with_progress(query, |_, _, _| {})
    }

    /// Execute a unified SearchQuery across a sub-range of games [start_game..end_game] in parallel with progress streaming
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
        crate::search::scid_adapter::ScidSearchAdapter::search_parallel_range_with_progress(
            query,
            self.entries(),
            self.names(),
            start_game,
            end_game,
            |entry| self.get_blob(entry).ok(),
            progress,
        )
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
