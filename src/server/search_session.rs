use crate::search::ScidMatchResult;
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
    pub created_at: Instant,
    pub duration_ms: u64,
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

    /// Clears all cached sessions
    pub fn clear(&mut self) {
        self.sessions.clear();
        self.query_to_id.clear();
    }
}
