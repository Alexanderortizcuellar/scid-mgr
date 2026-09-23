use chess_scid_rw::dates::date_to_pgn;
use chess_scid_rw::eco::eco_to_string;
use rayon::prelude::*;
use std::sync::Arc;

use super::core::ScidDatabaseWrapper;
use super::types::{result_code_to_str, GameFilter, GameSummary};

impl ScidDatabaseWrapper {
    pub fn clear_query_caches(&self) {
        if let Ok(mut g) = self.query_cache.lock() {
            *g = None;
        }
        if let Ok(mut g) = self.column_sort_cache.lock() {
            g.clear();
        }
    }

    pub fn get_player_ranks(&self) -> &[u32] {
        self.player_ranks.get_or_init(|| {
            let names = &self.names.players;
            let mut ranks = vec![0u32; names.len()];
            let mut ids: Vec<u32> = (0..names.len() as u32).collect();
            ids.par_sort_unstable_by(|&a, &b| names[a as usize].cmp(&names[b as usize]));
            for (rank, &id) in ids.iter().enumerate() {
                ranks[id as usize] = rank as u32;
            }
            ranks
        })
    }

    pub fn get_event_ranks(&self) -> &[u32] {
        self.event_ranks.get_or_init(|| {
            let names = &self.names.events;
            let mut ranks = vec![0u32; names.len()];
            let mut ids: Vec<u32> = (0..names.len() as u32).collect();
            ids.par_sort_unstable_by(|&a, &b| names[a as usize].cmp(&names[b as usize]));
            for (rank, &id) in ids.iter().enumerate() {
                ranks[id as usize] = rank as u32;
            }
            ranks
        })
    }

    pub fn get_site_ranks(&self) -> &[u32] {
        self.site_ranks.get_or_init(|| {
            let names = &self.names.sites;
            let mut ranks = vec![0u32; names.len()];
            let mut ids: Vec<u32> = (0..names.len() as u32).collect();
            ids.par_sort_unstable_by(|&a, &b| names[a as usize].cmp(&names[b as usize]));
            for (rank, &id) in ids.iter().enumerate() {
                ranks[id as usize] = rank as u32;
            }
            ranks
        })
    }

    pub fn get_round_ranks(&self) -> &[u32] {
        self.round_ranks.get_or_init(|| {
            let names = &self.names.rounds;
            let mut ranks = vec![0u32; names.len()];
            let mut ids: Vec<u32> = (0..names.len() as u32).collect();
            ids.par_sort_unstable_by(|&a, &b| names[a as usize].cmp(&names[b as usize]));
            for (rank, &id) in ids.iter().enumerate() {
                ranks[id as usize] = rank as u32;
            }
            ranks
        })
    }

    pub fn get_game_summary(&self, index: usize) -> Option<GameSummary> {
        let entry = self.entries.get(index)?;
        let white = self.names.player(entry.white_id).to_string();
        let black = self.names.player(entry.black_id).to_string();
        let event = self.names.event(entry.event_id).to_string();
        let site = self.names.site(entry.site_id).to_string();
        let round = self.names.round(entry.round_id).to_string();
        let result = result_code_to_str(entry.result).to_string();
        let eco = eco_to_string(entry.eco_code).unwrap_or_default();
        let date = date_to_pgn(entry.date);

        Some(GameSummary {
            id: index,
            white,
            white_elo: entry.white_elo,
            black,
            black_elo: entry.black_elo,
            result,
            eco,
            date,
            event,
            site,
            round,
            deleted: entry.deleted,
            non_standard_start: entry.non_standard_start,
            num_moves: 0,
            time_control: None,
        })
    }

    pub fn query_games_with_progress<F>(
        &self,
        filter: &GameFilter,
        page: usize,
        page_size: usize,
        progress: F,
    ) -> (Vec<GameSummary>, usize)
    where
        F: Fn(usize, usize, usize) + Sync,
    {
        // 0. Fast Column Sort Cache for Unfiltered Views (0.00ms lookup across whole table)
        let is_unfiltered = filter.is_empty()
            && filter.include_deleted.unwrap_or(true)
            && !filter.only_deleted.unwrap_or(false);

        if is_unfiltered {
            let col = filter.sort_by.as_deref().unwrap_or("id").to_lowercase();
            let is_asc = filter.sort_asc.unwrap_or(true);
            let total_matches = self.entries.len();

            if let Ok(guard) = self.column_sort_cache.lock() {
                if let Some(cached_asc) = guard.get(&col) {
                    let start = page * page_size;
                    if start >= total_matches {
                        return (Vec::new(), total_matches);
                    }
                    let end = usize::min(start + page_size, total_matches);
                    let summaries = if is_asc {
                        cached_asc[start..end]
                            .iter()
                            .filter_map(|&idx| self.get_game_summary(idx))
                            .collect()
                    } else {
                        (start..end)
                            .map(|i| cached_asc[total_matches - 1 - i])
                            .filter_map(|idx| self.get_game_summary(idx))
                            .collect()
                    };
                    return (summaries, total_matches);
                }
            }
        }

        // 1. Fast Query Cache: If identical filter is queried for subsequent pages, return instantly (0.00ms)
        // If search criteria match but sort changed, sort in-memory instantly without rescanning disk!
        if let Ok(mut guard) = self.query_cache.lock() {
            if let Some((ref cached_filter, ref cached_indices)) = *guard {
                if cached_filter == filter {
                    let total_matches = cached_indices.len();
                    let start = page * page_size;
                    if start >= total_matches {
                        return (Vec::new(), total_matches);
                    }
                    let end = usize::min(start + page_size, total_matches);
                    let summaries = cached_indices[start..end]
                        .iter()
                        .filter_map(|&idx| self.get_game_summary(idx))
                        .collect();
                    return (summaries, total_matches);
                } else if cached_filter.same_search_criteria(filter) {
                    let mut sorted_indices = cached_indices.clone();
                    if cached_filter.sort_by == filter.sort_by
                        && cached_filter.sort_asc != filter.sort_asc
                    {
                        sorted_indices.reverse();
                    } else {
                        self.sort_indices(
                            &mut sorted_indices,
                            filter.sort_by.as_deref(),
                            filter.sort_asc,
                        );
                    }
                    let total_matches = sorted_indices.len();
                    let start = page * page_size;
                    let summaries = if start >= total_matches {
                        Vec::new()
                    } else {
                        let end = usize::min(start + page_size, total_matches);
                        sorted_indices[start..end]
                            .iter()
                            .filter_map(|&idx| self.get_game_summary(idx))
                            .collect()
                    };
                    *guard = Some((filter.clone(), sorted_indices));
                    return (summaries, total_matches);
                }
            }
        }

        let eco_filter = filter.eco.as_ref().map(|s| s.to_uppercase());
        let date_filter = filter.date.as_ref().map(|s| s.trim());
        let result_filter = filter.result.as_deref();
        let include_del = filter.include_deleted.unwrap_or(true);
        let only_del = filter.only_deleted.unwrap_or(false);

        // Pre-build name matching boolean bitsets in O(M) where M is unique names (~15k)
        let player_matches = filter.player.as_deref().and_then(|p| {
            if p.trim().is_empty() {
                None
            } else {
                let pat = p.to_lowercase();
                Some(
                    self.names
                        .players
                        .iter()
                        .map(|name| name.to_lowercase().contains(&pat))
                        .collect::<Vec<bool>>(),
                )
            }
        });

        let white_matches = filter.white.as_deref().and_then(|w| {
            if w.trim().is_empty() {
                None
            } else {
                let pat = w.to_lowercase();
                Some(
                    self.names
                        .players
                        .iter()
                        .map(|name| name.to_lowercase().contains(&pat))
                        .collect::<Vec<bool>>(),
                )
            }
        });

        let black_matches = filter.black.as_deref().and_then(|b| {
            if b.trim().is_empty() {
                None
            } else {
                let pat = b.to_lowercase();
                Some(
                    self.names
                        .players
                        .iter()
                        .map(|name| name.to_lowercase().contains(&pat))
                        .collect::<Vec<bool>>(),
                )
            }
        });

        let event_matches = filter.event.as_deref().and_then(|e| {
            if e.trim().is_empty() {
                None
            } else {
                let pat = e.to_lowercase();
                Some(
                    self.names
                        .events
                        .iter()
                        .map(|name| name.to_lowercase().contains(&pat))
                        .collect::<Vec<bool>>(),
                )
            }
        });

        let site_matches = filter.site.as_deref().and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                let pat = s.to_lowercase();
                Some(
                    self.names
                        .sites
                        .iter()
                        .map(|name| name.to_lowercase().contains(&pat))
                        .collect::<Vec<bool>>(),
                )
            }
        });

        // ⚡ Candidate Game IDs from Position Search / .pos.idx Accelerator
        let mut candidate_ids: Option<Vec<usize>> = None;
        if let Some(f) = filter.fen.as_deref() {
            let trimmed = f.trim();
            if !trimmed.is_empty() {
                if let Ok(res) = self.search_position_with_progress(
                    trimmed,
                    filter.turn.as_deref(),
                    filter.match_mode.as_deref(),
                    filter.max_ply,
                    &progress,
                ) {
                    candidate_ids = Some(res.matches.into_iter().map(|m| m.game_id).collect());
                } else {
                    candidate_ids = Some(Vec::new());
                }
            }
        }

        let mat_matches = filter.material.as_ref().and_then(|m| {
            self.search_material_with_progress(m, &progress)
                .ok()
                .map(|vec| {
                    vec.into_iter()
                        .collect::<std::collections::HashSet<usize>>()
                })
        });

        let cql_matches = filter
            .cql
            .as_deref()
            .or(filter.query.as_deref())
            .and_then(|q_str| {
                let trimmed = q_str.trim();
                if trimmed.is_empty() {
                    None
                } else if let Ok(q) = crate::search::QueryParser::parse_str(trimmed) {
                    let matches = match (filter.start_game, filter.end_game) {
                        (Some(s), Some(e)) => {
                            self.search_query_range_with_progress(&q, s, e, &progress)
                        }
                        (Some(s), None) => self.search_query_range_with_progress(
                            &q,
                            s,
                            self.entries.len(),
                            &progress,
                        ),
                        (None, Some(e)) => {
                            self.search_query_range_with_progress(&q, 0, e, &progress)
                        }
                        (None, None) => self.search_query_progress_helper(&q, &progress),
                    };
                    Some(
                        matches
                            .into_iter()
                            .map(|m| m.game_id)
                            .collect::<std::collections::HashSet<usize>>(),
                    )
                } else {
                    Some(std::collections::HashSet::new())
                }
            });

        if let Some(ref c_set) = cql_matches {
            if let Some(ref existing) = candidate_ids {
                candidate_ids = Some(
                    existing
                        .iter()
                        .copied()
                        .filter(|id| c_set.contains(id))
                        .collect(),
                );
            } else {
                candidate_ids = Some(c_set.iter().copied().collect());
            }
        }

        let has_filter = candidate_ids.is_some()
            || mat_matches.is_some()
            || cql_matches.is_some()
            || only_del
            || !include_del
            || result_filter.is_some()
            || eco_filter.is_some()
            || date_filter.is_some()
            || player_matches.is_some()
            || white_matches.is_some()
            || black_matches.is_some()
            || event_matches.is_some()
            || site_matches.is_some();

        let mut matched_indices: Vec<usize> = if let Some(ref c_ids) = candidate_ids {
            c_ids
                .par_iter()
                .filter_map(|&idx| {
                    if idx >= self.entries.len() {
                        return None;
                    }
                    let entry = &self.entries[idx];
                    if let Some(ref m_set) = mat_matches {
                        if !m_set.contains(&idx) {
                            return None;
                        }
                    }
                    if let Some(ref c_set) = cql_matches {
                        if !c_set.contains(&idx) {
                            return None;
                        }
                    }
                    if only_del {
                        if !entry.deleted {
                            return None;
                        }
                    } else if !include_del && entry.deleted {
                        return None;
                    }
                    if let Some(res) = result_filter {
                        if res != "All" && !res.is_empty() {
                            let actual_res = result_code_to_str(entry.result);
                            if actual_res != res {
                                return None;
                            }
                        }
                    }
                    if let Some(eco_prefix) = &eco_filter {
                        if !eco_prefix.is_empty() {
                            let actual_eco = eco_to_string(entry.eco_code).unwrap_or_default();
                            if !actual_eco.starts_with(eco_prefix) {
                                return None;
                            }
                        }
                    }
                    if let Some(date_pat) = date_filter {
                        if !date_pat.is_empty() {
                            let actual_date = date_to_pgn(entry.date);
                            if !actual_date.contains(date_pat) {
                                return None;
                            }
                        }
                    }
                    if let Some(ref m) = player_matches {
                        let w_ok =
                            (entry.white_id as usize) < m.len() && m[entry.white_id as usize];
                        let b_ok =
                            (entry.black_id as usize) < m.len() && m[entry.black_id as usize];
                        if !w_ok && !b_ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = white_matches {
                        let ok = (entry.white_id as usize) < m.len() && m[entry.white_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = black_matches {
                        let ok = (entry.black_id as usize) < m.len() && m[entry.black_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = event_matches {
                        let ok = (entry.event_id as usize) < m.len() && m[entry.event_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = site_matches {
                        let ok = (entry.site_id as usize) < m.len() && m[entry.site_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    Some(idx)
                })
                .collect()
        } else if has_filter {
            self.entries
                .par_iter()
                .enumerate()
                .filter_map(|(idx, entry)| {
                    if let Some(ref m_set) = mat_matches {
                        if !m_set.contains(&idx) {
                            return None;
                        }
                    }
                    if let Some(ref c_set) = cql_matches {
                        if !c_set.contains(&idx) {
                            return None;
                        }
                    }
                    if only_del {
                        if !entry.deleted {
                            return None;
                        }
                    } else if !include_del && entry.deleted {
                        return None;
                    }
                    if let Some(res) = result_filter {
                        if res != "All" && !res.is_empty() {
                            let actual_res = result_code_to_str(entry.result);
                            if actual_res != res {
                                return None;
                            }
                        }
                    }
                    if let Some(eco_prefix) = &eco_filter {
                        if !eco_prefix.is_empty() {
                            let actual_eco = eco_to_string(entry.eco_code).unwrap_or_default();
                            if !actual_eco.starts_with(eco_prefix) {
                                return None;
                            }
                        }
                    }
                    if let Some(date_pat) = date_filter {
                        if !date_pat.is_empty() {
                            let actual_date = date_to_pgn(entry.date);
                            if !actual_date.contains(date_pat) {
                                return None;
                            }
                        }
                    }
                    if let Some(ref m) = player_matches {
                        let w_ok =
                            (entry.white_id as usize) < m.len() && m[entry.white_id as usize];
                        let b_ok =
                            (entry.black_id as usize) < m.len() && m[entry.black_id as usize];
                        if !w_ok && !b_ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = white_matches {
                        let ok = (entry.white_id as usize) < m.len() && m[entry.white_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = black_matches {
                        let ok = (entry.black_id as usize) < m.len() && m[entry.black_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = event_matches {
                        let ok = (entry.event_id as usize) < m.len() && m[entry.event_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    if let Some(ref m) = site_matches {
                        let ok = (entry.site_id as usize) < m.len() && m[entry.site_id as usize];
                        if !ok {
                            return None;
                        }
                    }
                    Some(idx)
                })
                .collect()
        } else {
            (0..self.entries.len()).collect()
        };

        // Ultra-Fast Parallel Multi-Field Sorting
        self.sort_indices(
            &mut matched_indices,
            filter.sort_by.as_deref(),
            filter.sort_asc,
        );

        let total_matches = matched_indices.len();
        let start = page * page_size;

        if let Ok(mut guard) = self.query_cache.lock() {
            *guard = Some((filter.clone(), matched_indices.clone()));
        }

        // Cache full column permutation if unfiltered
        if is_unfiltered {
            let col = filter.sort_by.as_deref().unwrap_or("id").to_lowercase();
            let is_asc = filter.sort_asc.unwrap_or(true);
            let asc_indices = if is_asc {
                matched_indices.clone()
            } else {
                let mut rev = matched_indices.clone();
                rev.reverse();
                rev
            };
            if let Ok(mut c_guard) = self.column_sort_cache.lock() {
                c_guard.insert(col, Arc::new(asc_indices));
            }
        }

        if start >= total_matches {
            return (Vec::new(), total_matches);
        }

        let end = usize::min(start + page_size, total_matches);
        let summaries = matched_indices[start..end]
            .iter()
            .filter_map(|&idx| self.get_game_summary(idx))
            .collect();

        (summaries, total_matches)
    }

    pub fn query_games(
        &self,
        filter: &GameFilter,
        page: usize,
        page_size: usize,
    ) -> (Vec<GameSummary>, usize) {
        self.query_games_with_progress(filter, page, page_size, |_, _, _| {})
    }

    pub fn get_cached_query_indices(&self) -> Option<Vec<usize>> {
        if let Ok(guard) = self.query_cache.lock() {
            guard.as_ref().map(|(_, indices)| indices.clone())
        } else {
            None
        }
    }
}
