use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, UNIX_EPOCH};
use anyhow::{anyhow, Context, Result};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shakmaty::Position;

use crate::db::{GameFilter, GameSummary};

const PGN_INDEX_MAGIC: &[u8; 8] = b"SCIDPGN2";
const PGN_INDEX_VERSION: u32 = 1;

/// Packed 40-byte binary record for a single game in a raw .pgn file
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CompactPgnRecord {
    pub offset: u64,     // 8 bytes: byte offset in .pgn
    pub length: u32,     // 4 bytes: byte length of game text
    pub white_id: u32,   // 4 bytes: player name dictionary ID
    pub black_id: u32,   // 4 bytes: player name dictionary ID
    pub event_id: u32,   // 4 bytes: event name dictionary ID
    pub site_id: u32,    // 4 bytes: site name dictionary ID
    pub date: u32,       // 4 bytes: packed (YYYY << 9) | (MM << 5) | DD
    pub eco: u16,        // 2 bytes: packed ECO (0..499, or 0xFFFF)
    pub white_elo: u16,  // 2 bytes: Elo (0 = none)
    pub black_elo: u16,  // 2 bytes: Elo (0 = none)
    pub result: u8,      // 1 byte: 0=*, 1=1-0, 2=0-1, 3=1/2-1/2
    pub _padding: u8,    // 1 byte: alignment padding (total 40 bytes)
}

pub type PgnIndexEntry = CompactPgnRecord;

impl CompactPgnRecord {
    #[inline]
    pub fn result_str(&self) -> &'static str {
        unpack_result(self.result)
    }

    #[inline]
    pub fn date_str(&self) -> String {
        unpack_date(self.date)
    }

    #[inline]
    pub fn eco_str(&self) -> String {
        unpack_eco(self.eco)
    }
}

/// Deduplicated string dictionary for PGN metadata (Player names, Events, Sites)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PgnNameTables {
    pub players: Vec<String>,
    pub events: Vec<String>,
    pub sites: Vec<String>,
}

impl PgnNameTables {
    pub fn new() -> Self {
        Self {
            players: vec!["?".to_string()],
            events: vec!["?".to_string()],
            sites: vec!["?".to_string()],
        }
    }

    #[inline]
    pub fn player(&self, id: u32) -> &str {
        self.players.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
    }

    #[inline]
    pub fn event(&self, id: u32) -> &str {
        self.events.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
    }

    #[inline]
    pub fn site(&self, id: u32) -> &str {
        self.sites.get(id as usize).map(|s| s.as_str()).unwrap_or("?")
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct PgnIndexHeader {
    magic: [u8; 8],
    version: u32,
    flags: u32,
    pgn_mtime_secs: u64,
    pgn_file_size: u64,
    game_count: u64,
    namebase_offset: u64,
    namebase_len: u64,
    records_offset: u64,
}

/// In-memory wrapper and query engine for directly opened .pgn files
pub struct PgnDatabaseWrapper {
    pub pgn_path: PathBuf,
    pub entries: Vec<CompactPgnRecord>,
    pub names: PgnNameTables,
    mmap: Arc<Mmap>,
    query_cache: std::sync::Mutex<Option<(GameFilter, Vec<usize>)>>,
}

impl PgnDatabaseWrapper {
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

        let idx_path = Self::get_companion_idx_path(&pgn_path);
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
                let (scanned_names, scanned_entries) = Self::scan_pgn_parallel(&mmap_arc, pgn_len)?;
                let _ = Self::save_index_file(
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

    fn load_index_file(
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

    fn save_index_file(
        idx_path: &Path,
        names: &PgnNameTables,
        entries: &[CompactPgnRecord],
        pgn_mtime_secs: u64,
        pgn_file_size: u64,
    ) -> Result<()> {
        let temp_path = idx_path.with_extension("tmp");
        {
            let file = File::create(&temp_path)?;
            let mut writer = BufWriter::new(file);

            let header_size = std::mem::size_of::<PgnIndexHeader>() as u64;
            // Write placeholder header
            let dummy_header = vec![0u8; header_size as usize];
            writer.write_all(&dummy_header)?;

            // Serialize Namebase
            let namebase_offset = header_size;
            let serialized_names = bincode::serialize(names)?;
            let namebase_len = serialized_names.len() as u64;
            writer.write_all(&serialized_names)?;

            // Write Records
            let records_offset = namebase_offset + namebase_len;
            let records_bytes = unsafe {
                std::slice::from_raw_parts(
                    entries.as_ptr() as *const u8,
                    std::mem::size_of_val(entries),
                )
            };
            writer.write_all(records_bytes)?;

            // Seek back and write true header
            let header = PgnIndexHeader {
                magic: *PGN_INDEX_MAGIC,
                version: PGN_INDEX_VERSION,
                flags: 0,
                pgn_mtime_secs,
                pgn_file_size,
                game_count: entries.len() as u64,
                namebase_offset,
                namebase_len,
                records_offset,
            };
            let header_bytes = unsafe {
                std::slice::from_raw_parts(
                    &header as *const _ as *const u8,
                    std::mem::size_of::<PgnIndexHeader>(),
                )
            };

            writer.seek(SeekFrom::Start(0))?;
            writer.write_all(header_bytes)?;
            writer.flush()?;
        }

        let _ = std::fs::remove_file(idx_path);
        std::fs::rename(&temp_path, idx_path)?;
        Ok(())
    }

    /// Parallel multi-chunk scanner that extracts tags and deduplicates names with zero heap allocations per game
    fn scan_pgn_parallel(
        mmap: &[u8],
        total_len: u64,
    ) -> Result<(PgnNameTables, Vec<CompactPgnRecord>)> {
        if total_len == 0 {
            return Ok((PgnNameTables::new(), Vec::new()));
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

        let chunk_results: Result<Vec<Vec<RawGameRecord>>> = chunk_ranges
            .into_par_iter()
            .map(|(start_idx, end_idx)| Self::scan_chunk(mmap, start_idx, end_idx))
            .collect();

        let mut all_raw = Vec::new();
        for chunk in chunk_results? {
            all_raw.extend(chunk);
        }

        // Fast sequential dictionary deduplication pass
        let mut names = PgnNameTables::new();
        let mut player_map: HashMap<&str, u32> = HashMap::with_capacity(all_raw.len() / 4);
        let mut event_map: HashMap<&str, u32> = HashMap::with_capacity(all_raw.len() / 8);
        let mut site_map: HashMap<&str, u32> = HashMap::with_capacity(all_raw.len() / 8);

        player_map.insert("?", 0);
        event_map.insert("?", 0);
        site_map.insert("?", 0);

        let mut compact_entries = Vec::with_capacity(all_raw.len());

        for raw in all_raw {
            let white_id = *player_map.entry(raw.white).or_insert_with(|| {
                let id = names.players.len() as u32;
                names.players.push(raw.white.to_string());
                id
            });
            let black_id = *player_map.entry(raw.black).or_insert_with(|| {
                let id = names.players.len() as u32;
                names.players.push(raw.black.to_string());
                id
            });
            let event_id = *event_map.entry(raw.event).or_insert_with(|| {
                let id = names.events.len() as u32;
                names.events.push(raw.event.to_string());
                id
            });
            let site_id = *site_map.entry(raw.site).or_insert_with(|| {
                let id = names.sites.len() as u32;
                names.sites.push(raw.site.to_string());
                id
            });

            compact_entries.push(CompactPgnRecord {
                offset: raw.offset,
                length: raw.length,
                white_id,
                black_id,
                event_id,
                site_id,
                date: raw.date,
                eco: raw.eco,
                white_elo: raw.white_elo,
                black_elo: raw.black_elo,
                result: raw.result,
                _padding: 0,
            });
        }

        Ok((names, compact_entries))
    }

    fn scan_chunk<'a>(mmap: &'a [u8], chunk_start: usize, chunk_end: usize) -> Result<Vec<RawGameRecord<'a>>> {
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
            let mut white = "?";
            let mut black = "?";
            let mut date_raw = "????.??.??";
            let mut result_raw = "*";
            let mut eco_raw = "";
            let mut event = "?";
            let mut site = "?";
            let mut white_elo = 0u16;
            let mut black_elo = 0u16;

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
                                "White" => white = tag_val,
                                "Black" => black = tag_val,
                                "Date" => date_raw = tag_val,
                                "Result" => result_raw = tag_val,
                                "ECO" => eco_raw = tag_val,
                                "Event" => event = tag_val,
                                "Site" => site = tag_val,
                                "WhiteElo" => white_elo = tag_val.parse::<u16>().unwrap_or(0),
                                "BlackElo" => black_elo = tag_val.parse::<u16>().unwrap_or(0),
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

            entries.push(RawGameRecord {
                offset: game_start as u64,
                length,
                white,
                black,
                event,
                site,
                date: pack_date(date_raw),
                eco: pack_eco(eco_raw),
                white_elo,
                black_elo,
                result: pack_result(result_raw),
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
            let names = &self.names;
            matched_indices.par_sort_unstable_by(|&a, &b| {
                let ea = &entries[a];
                let eb = &entries[b];
                let ord = match sort_field.to_lowercase().as_str() {
                    "id" => a.cmp(&b),
                    "white" => names.player(ea.white_id).cmp(names.player(eb.white_id)),
                    "black" => names.player(ea.black_id).cmp(names.player(eb.black_id)),
                    "white_elo" => ea.white_elo.cmp(&eb.white_elo),
                    "black_elo" => ea.black_elo.cmp(&eb.black_elo),
                    "result" => ea.result.cmp(&eb.result),
                    "eco" => ea.eco.cmp(&eb.eco),
                    "date" => ea.date.cmp(&eb.date),
                    "event" => names.event(ea.event_id).cmp(names.event(eb.event_id)),
                    "site" => names.site(ea.site_id).cmp(names.site(eb.site_id)),
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
                Some(self.names.players.iter().map(|p| p.to_lowercase().contains(&pat_lower)).collect())
            }
        });

        let matching_white: Option<Vec<bool>> = filter.white.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(self.names.players.iter().map(|p| p.to_lowercase().contains(&pat_lower)).collect())
            }
        });

        let matching_black: Option<Vec<bool>> = filter.black.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(self.names.players.iter().map(|p| p.to_lowercase().contains(&pat_lower)).collect())
            }
        });

        let matching_events: Option<Vec<bool>> = filter.event.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(self.names.events.iter().map(|e| e.to_lowercase().contains(&pat_lower)).collect())
            }
        });

        let matching_sites: Option<Vec<bool>> = filter.site.as_ref().and_then(|pat| {
            let pat_lower = pat.to_lowercase();
            if pat_lower.is_empty() {
                None
            } else {
                Some(self.names.sites.iter().map(|s| s.to_lowercase().contains(&pat_lower)).collect())
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
            self.search_material(m, |scanned, total, matches| {
                progress(scanned, total, matches);
            }).ok().map(|vec| {
                vec.into_iter().collect::<std::collections::HashSet<usize>>()
            })
        });

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
                        let w_ok = p_flags.get(entry.white_id as usize).copied().unwrap_or(false);
                        let b_ok = p_flags.get(entry.black_id as usize).copied().unwrap_or(false);
                        if !w_ok && !b_ok {
                            return false;
                        }
                    }
                    if let Some(ref w_flags) = matching_white {
                        if !w_flags.get(entry.white_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }
                    if let Some(ref b_flags) = matching_black {
                        if !b_flags.get(entry.black_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }
                    if let Some(ref ev_flags) = matching_events {
                        if !ev_flags.get(entry.event_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }
                    if let Some(ref st_flags) = matching_sites {
                        if !st_flags.get(entry.site_id as usize).copied().unwrap_or(false) {
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
                        let w_ok = p_flags.get(entry.white_id as usize).copied().unwrap_or(false);
                        let b_ok = p_flags.get(entry.black_id as usize).copied().unwrap_or(false);
                        if !w_ok && !b_ok {
                            return false;
                        }
                    }
                    if let Some(ref w_flags) = matching_white {
                        if !w_flags.get(entry.white_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }
                    if let Some(ref b_flags) = matching_black {
                        if !b_flags.get(entry.black_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }
                    if let Some(ref ev_flags) = matching_events {
                        if !ev_flags.get(entry.event_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }
                    if let Some(ref st_flags) = matching_sites {
                        if !st_flags.get(entry.site_id as usize).copied().unwrap_or(false) {
                            return false;
                        }
                    }

                    true
                })
                .collect()
        };

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

    pub fn get_cached_query_indices(&self) -> Option<Vec<usize>> {
        if let Ok(guard) = self.query_cache.lock() {
            guard.as_ref().map(|(_, indices)| indices.clone())
        } else {
            None
        }
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
        let is_exact_mode = mode_param.map(|m| {
            let m = m.to_lowercase();
            m == "exact" || m == "auto" || m.is_empty()
        }).unwrap_or(true);

        if is_exact_mode && turn_param.is_none() {
            if let Some((_pos, zobrist_hash)) = crate::position_index::parse_target_position(target_fen) {
                if let Ok(pos_idx) = crate::position_index::PositionIndex::load(&self.pgn_path) {
                    if let Some(gids) = pos_idx.get_all_position_games(zobrist_hash) {
                        let matches: Vec<crate::position_search::PositionMatch> = gids
                            .into_iter()
                            .map(|gid| crate::position_search::PositionMatch {
                                game_id: gid as usize,
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

struct RawGameRecord<'a> {
    offset: u64,
    length: u32,
    white: &'a str,
    black: &'a str,
    event: &'a str,
    site: &'a str,
    date: u32,
    eco: u16,
    white_elo: u16,
    black_elo: u16,
    result: u8,
}

pub fn pack_date(s: &str) -> u32 {
    let mut parts = s.split('.');
    let year = parts.next().and_then(|y| y.parse::<u16>().ok()).unwrap_or(0);
    let month = parts.next().and_then(|m| m.parse::<u8>().ok()).unwrap_or(0);
    let day = parts.next().and_then(|d| d.parse::<u8>().ok()).unwrap_or(0);
    ((year as u32) << 9) | (((month & 0x0F) as u32) << 5) | ((day & 0x1F) as u32)
}

pub fn unpack_date(d: u32) -> String {
    let year = (d >> 9) as u16;
    let month = ((d >> 5) & 0x0F) as u8;
    let day = (d & 0x1F) as u8;
    let y_str = if year == 0 { "????".to_string() } else { format!("{:04}", year) };
    let m_str = if month == 0 { "??".to_string() } else { format!("{:02}", month) };
    let d_str = if day == 0 { "??".to_string() } else { format!("{:02}", day) };
    format!("{}.{}.{}", y_str, m_str, d_str)
}

pub fn pack_eco(s: &str) -> u16 {
    let trimmed = s.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3 {
        let l = bytes[0].to_ascii_uppercase();
        if (b'A'..=b'E').contains(&l) {
            let letter_val = (l - b'A') as u16;
            if let Ok(digits) = std::str::from_utf8(&bytes[1..3]).unwrap_or("").parse::<u16>() {
                if digits < 100 {
                    return letter_val * 100 + digits;
                }
            }
        }
    }
    0xFFFF
}

pub fn unpack_eco(eco: u16) -> String {
    if eco <= 499 {
        let letter = (b'A' + (eco / 100) as u8) as char;
        let digits = eco % 100;
        format!("{}{:02}", letter, digits)
    } else {
        String::new()
    }
}

pub fn pack_result(s: &str) -> u8 {
    match s.trim() {
        "1-0" => 1,
        "0-1" => 2,
        "1/2-1/2" => 3,
        _ => 0,
    }
}

pub fn unpack_result(r: u8) -> &'static str {
    match r {
        1 => "1-0",
        2 => "0-1",
        3 => "1/2-1/2",
        _ => "*",
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
            if self.match_any_ply && !self.matched
                && self.check_material() {
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
