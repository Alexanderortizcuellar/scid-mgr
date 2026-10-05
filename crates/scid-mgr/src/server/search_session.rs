use crate::search::ScidMatchResult;
use crate::server::DatabaseBackend;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static NEXT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Target consumer/owner of the search session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOwner {
    #[serde(rename = "main")]
    Main,
    #[serde(rename = "reference")]
    Reference,
}

/// Board position matching criteria mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionMatchMode {
    Exact,
    Placement,
    Material,
}

/// Structured query identity and criteria encapsulated inside a session
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SessionQuery {
    PurePosition {
        fen: String,
        match_mode: PositionMatchMode,
        max_ply: Option<usize>,
    },
    FilteredPosition {
        fen: String,
        match_mode: PositionMatchMode,
        max_ply: Option<usize>,
        filter: Box<crate::db::GameFilter>,
    },
    HeaderSearch {
        filter: Box<crate::db::GameFilter>,
    },
    CqlSearch {
        query: String,
    },
    General {
        description: String,
    },
}

/// A cached search session containing the results of an executed search or position lookup
#[derive(Debug, Clone)]
pub struct SearchSession {
    pub search_id: String,
    pub db_key: String,
    pub owner: SessionOwner,
    pub query: SessionQuery,
    pub query_str: String,
    pub total_searched: usize,
    pub matches: Vec<ScidMatchResult>,
    pub sorted_cache: HashMap<(Option<String>, bool), Vec<ScidMatchResult>>,
    pub created_at: Instant,
    pub last_accessed: Instant,
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

/// In-memory manager for search sessions with isolated main table filter and LRU-capped reference sessions
#[derive(Debug)]
pub struct SearchSessionManager {
    pub main_session: Option<SearchSession>,
    pub reference_sessions: HashMap<String, SearchSession>,
    pub reference_lru: VecDeque<String>,
    pub max_reference_sessions: usize,
    query_to_id: HashMap<(String, String), String>, // (db_key, normalized_query) -> search_id
}

impl Default for SearchSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchSessionManager {
    pub const DEFAULT_MAX_REFERENCE_SESSIONS: usize = 32;

    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_MAX_REFERENCE_SESSIONS)
    }

    pub fn with_capacity(max_reference_sessions: usize) -> Self {
        Self {
            main_session: None,
            reference_sessions: HashMap::new(),
            reference_lru: VecDeque::new(),
            max_reference_sessions: max_reference_sessions.max(1),
            query_to_id: HashMap::new(),
        }
    }

    /// Generates a unique database key based on file path and game count
    pub fn db_key(path: &Path, game_count: usize) -> String {
        format!("{}:{}", path.to_string_lossy(), game_count)
    }

    /// Looks up an existing cached search session for identical query on identical database state
    pub fn find_cached(&mut self, db_key: &str, query_str: &str) -> Option<&SearchSession> {
        let key = (db_key.to_string(), query_str.trim().to_string());
        if let Some(id) = self.query_to_id.get(&key).cloned() {
            self.get_session_mut(&id).map(|s| &*s)
        } else {
            None
        }
    }

    /// Stores a new search session defaulting to MainTable ownership
    pub fn create_session(
        &mut self,
        db_key: &str,
        query_str: &str,
        total_searched: usize,
        matches: Vec<ScidMatchResult>,
        duration_ms: u64,
    ) -> String {
        self.create_session_with_metadata(
            db_key,
            query_str,
            total_searched,
            matches,
            duration_ms,
            SessionOwner::Main,
            SessionQuery::General {
                description: query_str.to_string(),
            },
        )
    }

    /// Stores a new search session with full metadata (owner, structured query) and enforces LRU rules
    pub fn create_session_with_metadata(
        &mut self,
        db_key: &str,
        query_str: &str,
        total_searched: usize,
        matches: Vec<ScidMatchResult>,
        duration_ms: u64,
        owner: SessionOwner,
        query: SessionQuery,
    ) -> String {
        let count = NEXT_SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
        let prefix = match owner {
            SessionOwner::Main => "main",
            SessionOwner::Reference => "ref",
        };
        let search_id = format!("{}_{}", prefix, count);

        let now = Instant::now();
        let session = SearchSession {
            search_id: search_id.clone(),
            db_key: db_key.to_string(),
            owner,
            query,
            query_str: query_str.trim().to_string(),
            total_searched,
            matches,
            sorted_cache: HashMap::new(),
            created_at: now,
            last_accessed: now,
            duration_ms,
        };

        let cache_key = (db_key.to_string(), query_str.trim().to_string());
        self.query_to_id.insert(cache_key, search_id.clone());

        match owner {
            SessionOwner::Main => {
                // If replacing main session, remove old query mapping if different
                if let Some(old) = self.main_session.take() {
                    let old_key = (old.db_key, old.query_str);
                    if old_key != (db_key.to_string(), query_str.trim().to_string()) {
                        self.query_to_id.remove(&old_key);
                    }
                }
                self.main_session = Some(session);
            }
            SessionOwner::Reference => {
                // Enforce LRU eviction for reference explorer sessions
                while self.reference_sessions.len() >= self.max_reference_sessions {
                    if let Some(old_id) = self.reference_lru.pop_front() {
                        if let Some(evicted) = self.reference_sessions.remove(&old_id) {
                            let old_key = (evicted.db_key, evicted.query_str);
                            self.query_to_id.remove(&old_key);
                        }
                    } else {
                        break;
                    }
                }
                self.reference_lru.push_back(search_id.clone());
                self.reference_sessions.insert(search_id.clone(), session);
            }
        }

        search_id
    }

    /// Retrieves an existing search session across all pools
    pub fn get_session(&self, search_id: &str) -> Option<&SearchSession> {
        self.get_session_by_owner(search_id, None)
    }

    /// Retrieves an existing search session optionally constrained by expected owner
    pub fn get_session_by_owner(
        &self,
        search_id: &str,
        owner: Option<SessionOwner>,
    ) -> Option<&SearchSession> {
        match owner {
            Some(SessionOwner::Main) => self
                .main_session
                .as_ref()
                .filter(|s| s.search_id == search_id),
            Some(SessionOwner::Reference) => self.reference_sessions.get(search_id),
            None => {
                if let Some(ref main) = self.main_session {
                    if main.search_id == search_id {
                        return Some(main);
                    }
                }
                self.reference_sessions.get(search_id)
            }
        }
    }

    /// Retrieves a mutable reference to an existing search session across all pools
    pub fn get_session_mut(&mut self, search_id: &str) -> Option<&mut SearchSession> {
        self.get_session_mut_by_owner(search_id, None)
    }

    /// Retrieves a mutable reference to an existing search session optionally constrained by owner
    pub fn get_session_mut_by_owner(
        &mut self,
        search_id: &str,
        owner: Option<SessionOwner>,
    ) -> Option<&mut SearchSession> {
        let now = Instant::now();
        match owner {
            Some(SessionOwner::Main) => {
                if let Some(ref mut main) = self.main_session {
                    if main.search_id == search_id {
                        main.last_accessed = now;
                        return Some(main);
                    }
                }
                None
            }
            Some(SessionOwner::Reference) => {
                if self.reference_sessions.contains_key(search_id) {
                    self.reference_lru.retain(|id| id != search_id);
                    self.reference_lru.push_back(search_id.to_string());
                    let s = self.reference_sessions.get_mut(search_id)?;
                    s.last_accessed = now;
                    Some(s)
                } else {
                    None
                }
            }
            None => {
                if let Some(ref mut main) = self.main_session {
                    if main.search_id == search_id {
                        main.last_accessed = now;
                        return Some(main);
                    }
                }
                if self.reference_sessions.contains_key(search_id) {
                    self.reference_lru.retain(|id| id != search_id);
                    self.reference_lru.push_back(search_id.to_string());
                    let s = self.reference_sessions.get_mut(search_id)?;
                    s.last_accessed = now;
                    Some(s)
                } else {
                    None
                }
            }
        }
    }

    /// Clears all cached sessions across both pools
    pub fn clear(&mut self) {
        self.main_session = None;
        self.reference_sessions.clear();
        self.reference_lru.clear();
        self.query_to_id.clear();
    }
}
