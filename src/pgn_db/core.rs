use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use memmap2::Mmap;
use rayon::prelude::*;

use crate::db::{GameFilter, GameSummary};

use super::builder::{save_index_file, scan_pgn_parallel};
use super::types::{
    CompactPgnRecord, PgnIndexHeader, PgnNameTables, PGN_INDEX_MAGIC, PGN_INDEX_VERSION,
};

/// In-memory wrapper and query engine for directly opened .pgn files
pub struct PgnDatabaseWrapper {
    pub pgn_path: PathBuf,
    pub entries: Vec<CompactPgnRecord>,
    pub names: PgnNameTables,
    pub(crate) mmap: Arc<Mmap>,
    pub(crate) player_ranks: OnceLock<Vec<u32>>,
    pub(crate) event_ranks: OnceLock<Vec<u32>>,
    pub(crate) site_ranks: OnceLock<Vec<u32>>,
    pub(crate) query_cache: Mutex<Option<(GameFilter, Vec<usize>)>>,
    pub(crate) column_sort_cache: Mutex<HashMap<String, Arc<Vec<usize>>>>,
}

pub type PgnDatabase = PgnDatabaseWrapper;

impl PgnDatabaseWrapper {
    /// Determines companion `.pgn.idx` path for any PGN file
    pub fn companion_path<P: AsRef<Path>>(pgn_path: P) -> PathBuf {
        let p = pgn_path.as_ref();
        let name = format!(
            "{}.idx",
            p.file_name().unwrap_or_default().to_string_lossy()
        );
        let mut res = p.to_path_buf();
        res.set_file_name(name);
        res
    }

    /// Loads binary `.pgn.idx` index file if valid and fresh
    pub fn load_index_file(
        idx_path: &Path,
        expected_mtime: u64,
        expected_len: u64,
    ) -> Result<(PgnNameTables, Vec<CompactPgnRecord>)> {
        let mut file = File::open(idx_path)?;
        let header_size = std::mem::size_of::<PgnIndexHeader>();
        let mut header_buf = vec![0u8; header_size];
        file.read_exact(&mut header_buf)?;

        let header: PgnIndexHeader = unsafe { std::ptr::read(header_buf.as_ptr() as *const _) };

        if &header.magic != PGN_INDEX_MAGIC || header.version != PGN_INDEX_VERSION {
            return Err(anyhow!("Invalid PGN index header or unsupported version"));
        }
        if header.pgn_mtime_secs != expected_mtime || header.pgn_file_size != expected_len {
            return Err(anyhow!("PGN index is stale compared to PGN source file"));
        }

        // Read Namebase
        file.seek(SeekFrom::Start(header.namebase_offset))?;
        let mut namebase_buf = vec![0u8; header.namebase_len as usize];
        file.read_exact(&mut namebase_buf)?;
        let names: PgnNameTables = bincode::deserialize(&namebase_buf)?;

        // Read Records
        file.seek(SeekFrom::Start(header.records_offset))?;
        let game_count = header.game_count as usize;
        let mut records = vec![CompactPgnRecord::default(); game_count];
        let records_byte_len = game_count * std::mem::size_of::<CompactPgnRecord>();
        let records_slice = unsafe {
            std::slice::from_raw_parts_mut(records.as_mut_ptr() as *mut u8, records_byte_len)
        };
        file.read_exact(records_slice)?;

        Ok((names, records))
    }

    /// Opens a .pgn file directly. If a companion single-file `<file>.pgn.idx` exists and matches,
    /// it loads in a few milliseconds; otherwise it runs a parallel 1-pass index scan and caches.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let pgn_path = path.as_ref().to_path_buf();
        let file = File::open(&pgn_path)
            .with_context(|| format!("Failed to open PGN file: {}", pgn_path.display()))?;
        let metadata = file.metadata()?;
        let pgn_len = metadata.len();
        let pgn_mtime = metadata.modified()?;
        let pgn_mtime_secs = pgn_mtime
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mmap = unsafe { Mmap::map(&file)? };
        let mmap_arc = Arc::new(mmap);

        let idx_path = Self::companion_path(&pgn_path);
        let mut loaded_data = None;

        if idx_path.exists() {
            if let Ok(loaded) = Self::load_index_file(&idx_path, pgn_mtime_secs, pgn_len) {
                loaded_data = Some(loaded);
            }
        }

        let (names, entries) = match loaded_data {
            Some(d) => d,
            None => {
                let start = Instant::now();
                let (scanned_names, scanned_entries) = scan_pgn_parallel(&mmap_arc, pgn_len)?;
                let _ = save_index_file(
                    &idx_path,
                    &scanned_names,
                    &scanned_entries,
                    pgn_mtime_secs,
                    pgn_len,
                );
                let elapsed = start.elapsed();
                log::info!(
                    "Indexed {} games ({} players, {} events, {} sites) from PGN in {:.2}s",
                    scanned_entries.len(),
                    scanned_names.players.len(),
                    scanned_names.events.len(),
                    scanned_names.sites.len(),
                    elapsed.as_secs_f64()
                );
                (scanned_names, scanned_entries)
            }
        };

        Ok(Self {
            pgn_path,
            entries,
            names,
            mmap: mmap_arc,
            player_ranks: OnceLock::new(),
            event_ranks: OnceLock::new(),
            site_ranks: OnceLock::new(),
            query_cache: Mutex::new(None),
            column_sort_cache: Mutex::new(HashMap::new()),
        })
    }

    pub fn game_count(&self) -> usize {
        self.entries.len()
    }

    pub fn mmap_ref(&self) -> &Mmap {
        &self.mmap
    }

    /// Returns the exact raw PGN text directly from memory-mapped disk in 0.01 ms
    pub fn get_game_pgn(&self, index: usize) -> Result<String> {
        let entry = self.entries.get(index).ok_or_else(|| {
            anyhow!(
                "Game index {} out of range (total: {})",
                index,
                self.entries.len()
            )
        })?;

        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > self.mmap.len() || start >= end {
            return Err(anyhow!("Game slice out of bounds in PGN file"));
        }

        let slice = &self.mmap[start..end];
        let pgn_text = String::from_utf8_lossy(slice).to_string();
        Ok(pgn_text.trim().to_string())
    }

    pub fn get_summary(&self, idx: usize) -> GameSummary {
        let e = &self.entries[idx];
        GameSummary {
            id: idx,
            white: self.names.player(e.white_id).to_string(),
            black: self.names.player(e.black_id).to_string(),
            white_elo: e.white_elo,
            black_elo: e.black_elo,
            date: e.date_str(),
            result: e.result_str().to_string(),
            eco: e.eco_str(),
            event: self.names.event(e.event_id).to_string(),
            site: self.names.site(e.site_id).to_string(),
            round: String::new(),
            deleted: false,
            non_standard_start: false,
            num_moves: 0,
            time_control: None,
        }
    }

    /// Query and filter games with progress callback, in-memory sort caching, and pagination
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
        let is_unfiltered = filter.is_empty();

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
                            .map(|&idx| self.get_summary(idx))
                            .collect()
                    } else {
                        (start..end)
                            .map(|i| cached_asc[total_matches - 1 - i])
                            .map(|idx| self.get_summary(idx))
                            .collect()
                    };
                    return (summaries, total_matches);
                }
            }
        }

        // 1. Fast Query Cache: If identical filter is queried for subsequent pages, return instantly (0.00ms)
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
                        .map(|&idx| self.get_summary(idx))
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
                            .map(|&idx| self.get_summary(idx))
                            .collect()
                    };
                    *guard = Some((filter.clone(), sorted_indices));
                    return (summaries, total_matches);
                }
            }
        }

        let eco_filter = filter.eco.as_ref().map(|s| s.to_uppercase());
        let date_filter = filter.date.as_ref().map(|s| s.trim());
        let result_val: Option<u8> = match filter.result.as_deref() {
            Some("1-0") => Some(1),
            Some("0-1") => Some(2),
            Some("1/2-1/2") => Some(3),
            Some("*") => Some(0),
            _ => None,
        };

        // Fast Name ID Pre-matching: Scan dictionary once, evaluate integer flags O(1) in game loop
        let matching_players: Option<Vec<bool>> = filter.player.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(
                    self.names
                        .players
                        .iter()
                        .map(|p| p.to_lowercase().contains(&pat_lower))
                        .collect(),
                )
            }
        });

        let matching_white: Option<Vec<bool>> = filter.white.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(
                    self.names
                        .players
                        .iter()
                        .map(|p| p.to_lowercase().contains(&pat_lower))
                        .collect(),
                )
            }
        });

        let matching_black: Option<Vec<bool>> = filter.black.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(
                    self.names
                        .players
                        .iter()
                        .map(|p| p.to_lowercase().contains(&pat_lower))
                        .collect(),
                )
            }
        });

        let matching_events: Option<Vec<bool>> = filter.event.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(
                    self.names
                        .events
                        .iter()
                        .map(|e| e.to_lowercase().contains(&pat_lower))
                        .collect(),
                )
            }
        });

        let matching_sites: Option<Vec<bool>> = filter.site.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(
                    self.names
                        .sites
                        .iter()
                        .map(|s| s.to_lowercase().contains(&pat_lower))
                        .collect(),
                )
            }
        });

        // ⚡ Candidate Game IDs from Position Search / .pos.idx Accelerator
        let mut candidate_ids: Option<Vec<usize>> = None;
        if let Some(ref f) = filter.fen {
            let trimmed = f.trim();
            if !trimmed.is_empty() {
                if let Ok(res) = self.search_position(
                    trimmed,
                    filter.turn.as_deref(),
                    filter.match_mode.as_deref(),
                    filter.max_ply,
                    |scanned, total, matches| {
                        progress(scanned, total, matches);
                    },
                ) {
                    candidate_ids = Some(res.matches.into_iter().map(|m| m.game_id).collect());
                } else {
                    candidate_ids = Some(Vec::new());
                }
            }
        }

        let mat_matches = filter.material.as_ref().and_then(|m| {
            self.search_material(m, &progress).ok().map(|vec| {
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
                        (None, None) => self.search_query_with_progress(&q, &progress),
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

        let mut matching_indices: Vec<usize> = if let Some(ref c_ids) = candidate_ids {
            c_ids
                .par_iter()
                .filter(|&&idx| {
                    if idx >= self.entries.len() {
                        return false;
                    }
                    if let Some(ref m_set) = mat_matches {
                        if !m_set.contains(&idx) {
                            return false;
                        }
                    }
                    if let Some(ref c_set) = cql_matches {
                        if !c_set.contains(&idx) {
                            return false;
                        }
                    }

                    let entry = &self.entries[idx];

                    if let Some(res) = result_val {
                        if entry.result != res {
                            return false;
                        }
                    }
                    if let Some(ref eco) = eco_filter {
                        if !eco.is_empty() && !entry.eco_str().starts_with(eco) {
                            return false;
                        }
                    }
                    if let Some(d) = date_filter {
                        if !d.is_empty() && !entry.date_str().starts_with(d) {
                            return false;
                        }
                    }
                    if let Some(ref p_flags) = matching_players {
                        let w_ok = p_flags
                            .get(entry.white_id as usize)
                            .copied()
                            .unwrap_or(false);
                        let b_ok = p_flags
                            .get(entry.black_id as usize)
                            .copied()
                            .unwrap_or(false);
                        if !w_ok && !b_ok {
                            return false;
                        }
                    }
                    if let Some(ref w_flags) = matching_white {
                        if !w_flags
                            .get(entry.white_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }
                    if let Some(ref b_flags) = matching_black {
                        if !b_flags
                            .get(entry.black_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }
                    if let Some(ref ev_flags) = matching_events {
                        if !ev_flags
                            .get(entry.event_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }
                    if let Some(ref st_flags) = matching_sites {
                        if !st_flags
                            .get(entry.site_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }

                    true
                })
                .copied()
                .collect()
        } else {
            (0..self.entries.len())
                .into_par_iter()
                .filter(|&idx| {
                    if let Some(ref m_set) = mat_matches {
                        if !m_set.contains(&idx) {
                            return false;
                        }
                    }
                    if let Some(ref c_set) = cql_matches {
                        if !c_set.contains(&idx) {
                            return false;
                        }
                    }

                    let entry = &self.entries[idx];

                    if let Some(res) = result_val {
                        if entry.result != res {
                            return false;
                        }
                    }
                    if let Some(ref eco) = eco_filter {
                        if !eco.is_empty() && !entry.eco_str().starts_with(eco) {
                            return false;
                        }
                    }
                    if let Some(d) = date_filter {
                        if !d.is_empty() && !entry.date_str().starts_with(d) {
                            return false;
                        }
                    }
                    if let Some(ref p_flags) = matching_players {
                        let w_ok = p_flags
                            .get(entry.white_id as usize)
                            .copied()
                            .unwrap_or(false);
                        let b_ok = p_flags
                            .get(entry.black_id as usize)
                            .copied()
                            .unwrap_or(false);
                        if !w_ok && !b_ok {
                            return false;
                        }
                    }
                    if let Some(ref w_flags) = matching_white {
                        if !w_flags
                            .get(entry.white_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }
                    if let Some(ref b_flags) = matching_black {
                        if !b_flags
                            .get(entry.black_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }
                    if let Some(ref ev_flags) = matching_events {
                        if !ev_flags
                            .get(entry.event_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }
                    if let Some(ref st_flags) = matching_sites {
                        if !st_flags
                            .get(entry.site_id as usize)
                            .copied()
                            .unwrap_or(false)
                        {
                            return false;
                        }
                    }

                    true
                })
                .collect()
        };

        let total_count = matching_indices.len();

        self.sort_indices(
            &mut matching_indices,
            filter.sort_by.as_deref(),
            filter.sort_asc,
        );

        if let Ok(mut guard) = self.query_cache.lock() {
            *guard = Some((filter.clone(), matching_indices.clone()));
        }

        // Cache full column permutation if unfiltered
        if is_unfiltered {
            let col = filter.sort_by.as_deref().unwrap_or("id").to_lowercase();
            let is_asc = filter.sort_asc.unwrap_or(true);
            let asc_indices = if is_asc {
                matching_indices.clone()
            } else {
                let mut rev = matching_indices.clone();
                rev.reverse();
                rev
            };
            if let Ok(mut c_guard) = self.column_sort_cache.lock() {
                c_guard.insert(col, Arc::new(asc_indices));
            }
        }

        let start_idx = page * page_size;
        if start_idx >= total_count {
            return (Vec::new(), total_count);
        }
        let end_idx = (start_idx + page_size).min(total_count);

        let games = matching_indices[start_idx..end_idx]
            .iter()
            .map(|&idx| self.get_summary(idx))
            .collect();

        (games, total_count)
    }

    /// Query and filter games with sorting and pagination
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
