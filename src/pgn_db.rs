use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use anyhow::{anyhow, Context, Result};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shakmaty::Position;

use crate::db::{GameFilter, GameSummary};

/// Compact binary index entry for a game inside a raw .pgn file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgnIndexEntry {
    pub offset: u64,
    pub length: u32,
    pub white: String,
    pub black: String,
    pub date: String,
    pub result: String,
    pub eco: String,
    pub event: String,
    pub site: String,
    pub white_elo: Option<u16>,
    pub black_elo: Option<u16>,
}

/// In-memory wrapper and query engine for directly opened .pgn files
pub struct PgnDatabaseWrapper {
    pub pgn_path: PathBuf,
    pub entries: Vec<PgnIndexEntry>,
    mmap: Arc<Mmap>,
    query_cache: std::sync::Mutex<Option<(GameFilter, Vec<usize>)>>,
}

impl PgnDatabaseWrapper {
    /// Opens a .pgn file directly. If a companion `<file>.pgn.idx` cache exists and is newer,
    /// it loads instantly; otherwise it runs a parallel 1-pass index scan and caches to disk.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let pgn_path = path.as_ref().to_path_buf();
        let file = File::open(&pgn_path)
            .with_context(|| format!("Failed to open PGN file: {}", pgn_path.display()))?;
        let metadata = file.metadata()?;
        let pgn_len = metadata.len();
        let pgn_mtime = metadata.modified()?;

        let mmap = unsafe { Mmap::map(&file)? };
        let mmap_arc = Arc::new(mmap);

        let idx_path = Self::get_companion_idx_path(&pgn_path);
        let mut entries = None;

        if idx_path.exists() {
            if let Ok(idx_meta) = idx_path.metadata() {
                if let Ok(idx_mtime) = idx_meta.modified() {
                    if idx_mtime >= pgn_mtime {
                        if let Ok(loaded) = Self::load_index_file(&idx_path) {
                            entries = Some(loaded);
                        }
                    }
                }
            }
        }

        let final_entries = match entries {
            Some(e) => e,
            None => {
                let start = Instant::now();
                let scanned = Self::scan_pgn_parallel(&mmap_arc, pgn_len)?;
                let _ = Self::save_index_file(&idx_path, &scanned);
                let elapsed = start.elapsed();
                log::info!(
                    "Indexed {} games from PGN in {:.2}s",
                    scanned.len(),
                    elapsed.as_secs_f64()
                );
                scanned
            }
        };

        Ok(Self {
            pgn_path,
            entries: final_entries,
            mmap: mmap_arc,
            query_cache: std::sync::Mutex::new(None),
        })
    }

    fn get_companion_idx_path(pgn_path: &Path) -> PathBuf {
        let mut p = pgn_path.to_path_buf();
        let name = format!(
            "{}.idx",
            pgn_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
        p.set_file_name(name);
        p
    }

    fn load_index_file(idx_path: &Path) -> Result<Vec<PgnIndexEntry>> {
        let file = File::open(idx_path)?;
        let reader = BufReader::new(file);
        let entries: Vec<PgnIndexEntry> = bincode::deserialize_from(reader)?;
        Ok(entries)
    }

    fn save_index_file(idx_path: &Path, entries: &[PgnIndexEntry]) -> Result<()> {
        let file = File::create(idx_path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, entries)?;
        Ok(())
    }

    /// Parallel multi-chunk scanner that finds all games and extracts tags in ~0.5-1s
    fn scan_pgn_parallel(mmap: &[u8], total_len: u64) -> Result<Vec<PgnIndexEntry>> {
        if total_len == 0 {
            return Ok(Vec::new());
        }

        let num_threads = rayon::current_num_threads().max(1);
        let chunk_size = ((total_len as usize) / num_threads).max(64 * 1024);

        // Find chunk boundaries aligned to game starts
        let mut chunk_starts = Vec::new();
        chunk_starts.push(0usize);

        for i in 1..num_threads {
            let rough_start = i * chunk_size;
            if rough_start >= mmap.len() {
                break;
            }
            // Advance to next '[Event ' at start of line
            let mut pos = rough_start;
            let mut found = false;
            while pos + 7 < mmap.len() {
                if (pos == 0 || mmap[pos - 1] == b'\n') && &mmap[pos..pos + 7] == b"[Event " {
                    found = true;
                    break;
                }
                pos += 1;
            }
            if found && pos < mmap.len() {
                chunk_starts.push(pos);
            }
        }
        chunk_starts.push(mmap.len());
        chunk_starts.dedup();

        let chunk_ranges: Vec<(usize, usize)> = chunk_starts
            .windows(2)
            .map(|w| (w[0], w[1]))
            .collect();

        let chunk_results: Result<Vec<Vec<PgnIndexEntry>>> = chunk_ranges
            .into_par_iter()
            .map(|(start_idx, end_idx)| {
                Self::scan_chunk(mmap, start_idx, end_idx)
            })
            .collect();

        let mut all_entries = Vec::new();
        for chunk in chunk_results? {
            all_entries.extend(chunk);
        }

        Ok(all_entries)
    }

    fn scan_chunk(mmap: &[u8], chunk_start: usize, chunk_end: usize) -> Result<Vec<PgnIndexEntry>> {
        let mut entries = Vec::new();
        let mut cursor = chunk_start;

        while cursor < chunk_end {
            // Find start of next game: line starting with '['
            while cursor < chunk_end {
                if (cursor == 0 || mmap[cursor - 1] == b'\n') && mmap[cursor] == b'[' {
                    break;
                }
                cursor += 1;
            }
            if cursor >= chunk_end {
                break;
            }

            let game_start = cursor;
            let mut white = String::from("?");
            let mut black = String::from("?");
            let mut date = String::from("????.??.??");
            let mut result = String::from("*");
            let mut eco = String::new();
            let mut event = String::from("?");
            let mut site = String::from("?");
            let mut white_elo = None;
            let mut black_elo = None;

            // Parse tag headers
            while cursor < mmap.len() {
                if mmap[cursor] != b'[' {
                    break;
                }
                let line_start = cursor;
                while cursor < mmap.len() && mmap[cursor] != b'\n' {
                    cursor += 1;
                }
                let line_bytes = &mmap[line_start..cursor];
                if cursor < mmap.len() && mmap[cursor] == b'\n' {
                    cursor += 1;
                }

                if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                    let trimmed = line_str.trim();
                    if trimmed.starts_with('[') && trimmed.ends_with(']') {
                        if let Some((tag_name, tag_val)) = Self::parse_tag(trimmed) {
                            match tag_name {
                                "White" => white = tag_val.to_string(),
                                "Black" => black = tag_val.to_string(),
                                "Date" => date = tag_val.to_string(),
                                "Result" => result = tag_val.to_string(),
                                "ECO" => eco = tag_val.to_string(),
                                "Event" => event = tag_val.to_string(),
                                "Site" => site = tag_val.to_string(),
                                "WhiteElo" => white_elo = tag_val.parse::<u16>().ok(),
                                "BlackElo" => black_elo = tag_val.parse::<u16>().ok(),
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Skip move text until next game start or end of file
            while cursor < mmap.len() {
                if (cursor == 0 || mmap[cursor - 1] == b'\n') && mmap[cursor] == b'[' {
                    // Check if this is a tag line (start of next game)
                    let mut tag_check = cursor;
                    while tag_check < mmap.len() && mmap[tag_check] != b'\n' && mmap[tag_check] != b']' {
                        tag_check += 1;
                    }
                    if tag_check < mmap.len() && mmap[tag_check] == b']' {
                        break;
                    }
                }
                cursor += 1;
            }

            let game_end = cursor;
            let length = (game_end - game_start) as u32;

            entries.push(PgnIndexEntry {
                offset: game_start as u64,
                length,
                white,
                black,
                date,
                result,
                eco,
                event,
                site,
                white_elo,
                black_elo,
            });
        }

        Ok(entries)
    }

    #[inline]
    fn parse_tag(line: &str) -> Option<(&str, &str)> {
        let inside = line.strip_prefix('[')?.strip_suffix(']')?.trim();
        let mut parts = inside.splitn(2, ' ');
        let tag_name = parts.next()?.trim();
        let raw_val = parts.next()?.trim();
        let val = raw_val.strip_prefix('"')?.strip_suffix('"')?;
        Some((tag_name, val))
    }

    pub fn game_count(&self) -> usize {
        self.entries.len()
    }

    pub fn mmap_ref(&self) -> &Mmap {
        &self.mmap
    }

    /// Returns the exact raw PGN text directly from memory-mapped disk in 0.01 ms
    pub fn get_game_pgn(&self, index: usize) -> Result<String> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| anyhow!("Game index {} out of range (total: {})", index, self.entries.len()))?;

        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        if end > self.mmap.len() || start >= end {
            return Err(anyhow!("Game slice out of bounds in PGN file"));
        }

        let slice = &self.mmap[start..end];
        let pgn_text = String::from_utf8_lossy(slice).to_string();
        Ok(pgn_text.trim().to_string())
    }

    pub fn sort_indices(&self, matched_indices: &mut [usize], sort_by: Option<&str>, sort_asc: Option<bool>) {
        if let Some(sort_field) = sort_by {
            let asc = sort_asc.unwrap_or(true);
            let entries = &self.entries;
            matched_indices.par_sort_unstable_by(|&a, &b| {
                let ea = &entries[a];
                let eb = &entries[b];
                let ord = match sort_field.to_lowercase().as_str() {
                    "id" => a.cmp(&b),
                    "white" => ea.white.cmp(&eb.white),
                    "black" => ea.black.cmp(&eb.black),
                    "white_elo" => ea.white_elo.unwrap_or(0).cmp(&eb.white_elo.unwrap_or(0)),
                    "black_elo" => ea.black_elo.unwrap_or(0).cmp(&eb.black_elo.unwrap_or(0)),
                    "result" => ea.result.cmp(&eb.result),
                    "eco" => ea.eco.cmp(&eb.eco),
                    "date" => ea.date.cmp(&eb.date),
                    "event" => ea.event.cmp(&eb.event),
                    "site" => ea.site.cmp(&eb.site),
                    _ => a.cmp(&b),
                };
                if asc {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }
    }

    pub fn get_summary(&self, idx: usize) -> GameSummary {
        let e = &self.entries[idx];
        GameSummary {
            id: idx,
            white: e.white.clone(),
            black: e.black.clone(),
            white_elo: e.white_elo.unwrap_or(0),
            black_elo: e.black_elo.unwrap_or(0),
            date: e.date.clone(),
            result: e.result.clone(),
            eco: e.eco.clone(),
            event: e.event.clone(),
            site: e.site.clone(),
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
                        .map(|&idx| self.get_summary(idx))
                        .collect();
                    return (summaries, total_matches);
                } else if cached_filter.same_search_criteria(filter) {
                    let mut sorted_indices = cached_indices.clone();
                    self.sort_indices(&mut sorted_indices, filter.sort_by.as_deref(), filter.sort_asc);
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
        let result_filter = filter.result.as_ref().map(|s| s.as_str());

        let player_pat = filter.player.as_ref().map(|s| s.to_lowercase());
        let white_pat = filter.white.as_ref().map(|s| s.to_lowercase());
        let black_pat = filter.black.as_ref().map(|s| s.to_lowercase());
        let event_pat = filter.event.as_ref().map(|s| s.to_lowercase());
        let site_pat = filter.site.as_ref().map(|s| s.to_lowercase());

        let pos_matches = filter.fen.as_deref().and_then(|f| {
            if f.trim().is_empty() {
                None
            } else {
                self.search_position(
                    f,
                    filter.turn.as_deref(),
                    filter.match_mode.as_deref(),
                    filter.max_ply,
                    |scanned, total, matches| {
                        progress(scanned, total, matches);
                    },
                ).ok().map(|res| {
                    res.matches.into_iter().map(|m| m.game_id).collect::<std::collections::HashSet<usize>>()
                })
            }
        });

        let mat_matches = filter.material.as_ref().and_then(|m| {
            self.search_material(m, |scanned, total, matches| {
                progress(scanned, total, matches);
            }).ok().map(|vec| {
                vec.into_iter().collect::<std::collections::HashSet<usize>>()
            })
        });

        let mut matching_indices: Vec<usize> = (0..self.entries.len())
            .into_par_iter()
            .filter(|&idx| {
                if let Some(ref p_set) = pos_matches {
                    if !p_set.contains(&idx) {
                        return false;
                    }
                }
                if let Some(ref m_set) = mat_matches {
                    if !m_set.contains(&idx) {
                        return false;
                    }
                }

                let entry = &self.entries[idx];

                if let Some(res) = result_filter {
                    if res != "All" && entry.result != res {
                        return false;
                    }
                }
                if let Some(eco) = &eco_filter {
                    if !eco.is_empty() && !entry.eco.to_uppercase().starts_with(eco) {
                        return false;
                    }
                }
                if let Some(d) = date_filter {
                    if !d.is_empty() && !entry.date.starts_with(d) {
                        return false;
                    }
                }
                if let Some(ref p) = player_pat {
                    if !p.is_empty() {
                        let w_match = entry.white.to_lowercase().contains(p);
                        let b_match = entry.black.to_lowercase().contains(p);
                        if !w_match && !b_match {
                            return false;
                        }
                    }
                }
                if let Some(ref w) = white_pat {
                    if !w.is_empty() && !entry.white.to_lowercase().contains(w) {
                        return false;
                    }
                }
                if let Some(ref b) = black_pat {
                    if !b.is_empty() && !entry.black.to_lowercase().contains(b) {
                        return false;
                    }
                }
                if let Some(ref ev) = event_pat {
                    if !ev.is_empty() && !entry.event.to_lowercase().contains(ev) {
                        return false;
                    }
                }
                if let Some(ref st) = site_pat {
                    if !st.is_empty() && !entry.site.to_lowercase().contains(st) {
                        return false;
                    }
                }

                true
            })
            .collect();

        let total_count = matching_indices.len();

        self.sort_indices(&mut matching_indices, filter.sort_by.as_deref(), filter.sort_asc);

        if let Ok(mut guard) = self.query_cache.lock() {
            *guard = Some((filter.clone(), matching_indices.clone()));
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
        let max_ply_val = max_ply.unwrap_or(500);

        let matcher = crate::position_search::parse_position_matcher(target_fen, turn_param, mode_param)?;

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
                    let slice = &self.mmap[entry.offset as usize..(entry.offset as usize + entry.length as usize)];
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
                    let slice = &self.mmap[entry.offset as usize..(entry.offset as usize + entry.length as usize)];
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
}

struct PositionFinder {
    matcher: crate::position_search::PositionTargetMatcher,
    found_ply: Option<usize>,
    ply: usize,
    max_ply: usize,
    pos: shakmaty::Chess,
}

impl PositionFinder {
    fn new(
        matcher: crate::position_search::PositionTargetMatcher,
        max_ply: usize,
    ) -> Self {
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
            if self.match_any_ply && !self.matched {
                if self.check_material() {
                    self.matched = true;
                }
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
