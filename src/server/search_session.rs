use crate::search::ScidMatchResult;
use crate::server::DatabaseBackend;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static NEXT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A cached search session containing the results of an executed CQL search
#[derive(Debug, Clone)]
pub struct SearchSession {
    pub search_id: String,
    pub db_key: String,
    pub query_str: String,
    pub total_searched: usize,
    pub matches: Vec<ScidMatchResult>,
    pub sorted_cache: HashMap<(Option<String>, bool), Vec<ScidMatchResult>>,
    pub created_at: Instant,
    pub duration_ms: u64,
}

impl SearchSession {
    /// Returns a paginated slice of matched games sorted according to the requested column
    pub fn get_sorted_slice(
        &mut self,
        sort_by: Option<&str>,
        sort_asc: bool,
        page: usize,
        page_size: usize,
        db: &DatabaseBackend,
    ) -> (&[ScidMatchResult], usize) {
        let total = self.matches.len();
        let start = page * page_size;
        if start >= total {
            return (&[], total);
        }
        let end = usize::min(start + page_size, total);

        let field = sort_by
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());

        // Default sorting: id / natural database order ascending
        if (field.is_none() || field.as_deref() == Some("id") || field.as_deref() == Some("index"))
            && sort_asc
        {
            return (&self.matches[start..end], total);
        }

        let cache_key = (field.clone(), sort_asc);
        if self.sorted_cache.contains_key(&cache_key) {
            let cached = self.sorted_cache.get(&cache_key).unwrap();
            return (&cached[start..end], total);
        }

        let mut sorted = self.matches.clone();

        match (db, field.as_deref()) {
            (DatabaseBackend::Scid(s), Some("date")) => {
                let entries = &s.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].date);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id].date.cmp(&entries[a.game_id].date)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("white_elo")) => {
                let entries = &s.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].white_elo);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id]
                            .white_elo
                            .cmp(&entries[a.game_id].white_elo)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("black_elo")) => {
                let entries = &s.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].black_elo);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id]
                            .black_elo
                            .cmp(&entries[a.game_id].black_elo)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("eco")) => {
                let entries = &s.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].eco_code);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id]
                            .eco_code
                            .cmp(&entries[a.game_id].eco_code)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("result")) => {
                let entries = &s.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].result);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id].result.cmp(&entries[a.game_id].result)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("white")) => {
                let entries = &s.entries;
                let ranks = s.get_player_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("black")) => {
                let entries = &s.entries;
                let ranks = s.get_player_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("event")) => {
                let entries = &s.entries;
                let ranks = s.get_event_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("site")) => {
                let entries = &s.entries;
                let ranks = s.get_site_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Scid(s), Some("round")) => {
                let entries = &s.entries;
                let ranks = s.get_round_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].round_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].round_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].round_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }

            // PGN Sorting
            (DatabaseBackend::Pgn(p), Some("date")) => {
                let entries = &p.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].date);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id].date.cmp(&entries[a.game_id].date)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("white_elo")) => {
                let entries = &p.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].white_elo);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id]
                            .white_elo
                            .cmp(&entries[a.game_id].white_elo)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("black_elo")) => {
                let entries = &p.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].black_elo);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id]
                            .black_elo
                            .cmp(&entries[a.game_id].black_elo)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("eco")) => {
                let entries = &p.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].eco);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id].eco.cmp(&entries[a.game_id].eco)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("result")) => {
                let entries = &p.entries;
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| entries[m.game_id].result);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        entries[b.game_id].result.cmp(&entries[a.game_id].result)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("white")) => {
                let entries = &p.entries;
                let ranks = p.get_player_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("black")) => {
                let entries = &p.entries;
                let ranks = p.get_player_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("event")) => {
                let entries = &p.entries;
                let ranks = p.get_event_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }
            (DatabaseBackend::Pgn(p), Some("site")) => {
                let entries = &p.entries;
                let ranks = p.get_site_ranks();
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        ranks
                            .get(entries[m.game_id].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let r_a = ranks
                            .get(entries[a.game_id].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        let r_b = ranks
                            .get(entries[b.game_id].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX);
                        r_b.cmp(&r_a)
                    });
                }
            }

            // Universal Match Details Sorting
            (_, Some("matches") | Some("match_count")) => {
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| m.match_details.match_count);
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        b.match_details
                            .match_count
                            .cmp(&a.match_details.match_count)
                    });
                }
            }
            (_, Some("first_ply") | Some("ply")) => {
                if sort_asc {
                    sorted.par_sort_unstable_by_key(|m| {
                        m.match_details
                            .matching_plies
                            .first()
                            .copied()
                            .unwrap_or(usize::MAX)
                    });
                } else {
                    sorted.par_sort_unstable_by(|a, b| {
                        let p_a = a.match_details.matching_plies.first().copied().unwrap_or(0);
                        let p_b = b.match_details.matching_plies.first().copied().unwrap_or(0);
                        p_b.cmp(&p_a)
                    });
                }
            }
            (_, Some("id") | Some("index")) => {
                if !sort_asc {
                    sorted.par_sort_unstable_by(|a, b| b.game_id.cmp(&a.game_id));
                } else {
                    sorted.par_sort_unstable_by_key(|m| m.game_id);
                }
            }
            _ => {}
        }

        self.sorted_cache.insert(cache_key.clone(), sorted);
        let cached = self.sorted_cache.get(&cache_key).unwrap();
        (&cached[start..end], total)
    }
}

/// In-memory manager for search sessions and query caching
#[derive(Debug, Default)]
pub struct SearchSessionManager {
    sessions: HashMap<String, SearchSession>,
    query_to_id: HashMap<(String, String), String>, // (db_key, normalized_query) -> search_id
}

impl SearchSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generates a unique database key based on file path and game count
    pub fn db_key(path: &Path, game_count: usize) -> String {
        format!("{}:{}", path.to_string_lossy(), game_count)
    }

    /// Looks up an existing cached search session for identical query on identical database state
    pub fn find_cached(&self, db_key: &str, query_str: &str) -> Option<&SearchSession> {
        let key = (db_key.to_string(), query_str.trim().to_string());
        if let Some(id) = self.query_to_id.get(&key) {
            self.sessions.get(id)
        } else {
            None
        }
    }

    /// Stores a new search session and caches it by (db_key, query_str)
    pub fn create_session(
        &mut self,
        db_key: &str,
        query_str: &str,
        total_searched: usize,
        matches: Vec<ScidMatchResult>,
        duration_ms: u64,
    ) -> String {
        let count = NEXT_SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        let search_id = format!("search_{}", count);

        let session = SearchSession {
            search_id: search_id.clone(),
            db_key: db_key.to_string(),
            query_str: query_str.trim().to_string(),
            total_searched,
            matches,
            sorted_cache: HashMap::new(),
            created_at: Instant::now(),
            duration_ms,
        };

        let cache_key = (db_key.to_string(), query_str.trim().to_string());
        self.query_to_id.insert(cache_key, search_id.clone());
        self.sessions.insert(search_id.clone(), session);

        search_id
    }

    /// Retrieves an existing search session by search_id
    pub fn get_session(&self, search_id: &str) -> Option<&SearchSession> {
        self.sessions.get(search_id)
    }

    /// Retrieves a mutable reference to an existing search session by search_id
    pub fn get_session_mut(&mut self, search_id: &str) -> Option<&mut SearchSession> {
        self.sessions.get_mut(search_id)
    }

    /// Clears all cached sessions
    pub fn clear(&mut self) {
        self.sessions.clear();
        self.query_to_id.clear();
    }
}
