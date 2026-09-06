use anyhow::{anyhow, Context, Result};
use chess_scid_rw::dates::date_to_pgn;
use chess_scid_rw::eco::eco_to_string;
use chess_scid_rw::entry::IndexEntry;
use chess_scid_rw::names::NameTables;
use chess_scid_rw::pgn_ingest;
use chess_scid_rw::{Si4Paths, Si5Paths};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScidFormat {
    Si4,
    Si5,
}

impl std::fmt::Display for ScidFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScidFormat::Si4 => write!(f, "si4"),
            ScidFormat::Si5 => write!(f, "si5"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: usize,
    pub white: String,
    pub white_elo: u16,
    pub black: String,
    pub black_elo: u16,
    pub result: String,
    pub eco: String,
    pub date: String,
    pub event: String,
    pub site: String,
    pub round: String,
    pub deleted: bool,
    pub non_standard_start: bool,
    pub num_moves: u32,
    pub time_control: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameFilter {
    pub player: Option<String>,
    pub white: Option<String>,
    pub black: Option<String>,
    pub result: Option<String>,
    pub eco: Option<String>,
    pub date: Option<String>,
    pub event: Option<String>,
    pub site: Option<String>,
    pub include_deleted: Option<bool>,
    pub only_deleted: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_asc: Option<bool>,
    pub fen: Option<String>,
    pub turn: Option<String>,
    pub match_mode: Option<String>,
    pub max_ply: Option<usize>,
    pub material: Option<crate::position_search::MaterialFilter>,
}

impl GameFilter {
    pub fn same_search_criteria(&self, other: &Self) -> bool {
        self.player == other.player
            && self.white == other.white
            && self.black == other.black
            && self.result == other.result
            && self.eco == other.eco
            && self.date == other.date
            && self.event == other.event
            && self.site == other.site
            && self.include_deleted == other.include_deleted
            && self.only_deleted == other.only_deleted
            && self.fen == other.fen
            && self.turn == other.turn
            && self.match_mode == other.match_mode
            && self.material == other.material
    }

    pub fn is_empty(&self) -> bool {
        self.player.as_deref().unwrap_or("").trim().is_empty()
            && self.white.as_deref().unwrap_or("").trim().is_empty()
            && self.black.as_deref().unwrap_or("").trim().is_empty()
            && self.result.as_deref().unwrap_or("").trim().is_empty()
            && self.eco.as_deref().unwrap_or("").trim().is_empty()
            && self.date.as_deref().unwrap_or("").trim().is_empty()
            && self.event.as_deref().unwrap_or("").trim().is_empty()
            && self.site.as_deref().unwrap_or("").trim().is_empty()
            && !self.only_deleted.unwrap_or(false)
            && self.material.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub format: ScidFormat,
    pub index_path: String,
    pub total_games: usize,
    pub active_games: usize,
    pub deleted_games: usize,
    pub players_count: usize,
    pub events_count: usize,
    pub sites_count: usize,
    pub rounds_count: usize,
    pub index_file_size: u64,
    pub namebase_file_size: u64,
    pub games_file_size: u64,
}

pub struct ScidDatabaseWrapper {
    format: ScidFormat,
    entries: Vec<IndexEntry>,
    names: NameTables,
    index_path: PathBuf,
    namebase_path: PathBuf,
    games_path: PathBuf,
    games_mmap: Option<Mmap>,
    pending_games: Vec<u8>,
    dirty: bool,
    player_ranks: std::sync::OnceLock<Vec<u32>>,
    event_ranks: std::sync::OnceLock<Vec<u32>>,
    site_ranks: std::sync::OnceLock<Vec<u32>>,
    round_ranks: std::sync::OnceLock<Vec<u32>>,
    query_cache: std::sync::Mutex<Option<(GameFilter, Vec<usize>)>>,
}

pub fn result_code_to_str(res: u8) -> &'static str {
    match res {
        1 => "1-0",
        2 => "0-1",
        3 => "1/2-1/2",
        _ => "*",
    }
}

pub fn detect_format_from_path(path: &Path) -> (ScidFormat, PathBuf) {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let lower_ext = ext.to_lowercase();

    if lower_ext == "si5" || lower_ext == "sn5" || lower_ext == "sg5" {
        (ScidFormat::Si5, path.with_extension("si5"))
    } else if lower_ext == "si4" || lower_ext == "sn4" || lower_ext == "sg4" {
        (ScidFormat::Si4, path.with_extension("si4"))
    } else if path.with_extension("si5").exists() {
        (ScidFormat::Si5, path.with_extension("si5"))
    } else if path.with_extension("si4").exists() {
        (ScidFormat::Si4, path.with_extension("si4"))
    } else {
        (ScidFormat::Si5, path.with_extension("si5"))
    }
}

impl ScidDatabaseWrapper {
    pub fn open(path: &Path) -> Result<Self> {
        let (format, index_path) = detect_format_from_path(path);
        let (namebase_path, games_path) = match format {
            ScidFormat::Si4 => {
                let p = Si4Paths::from_index_path(&index_path);
                (p.namebase, p.games)
            }
            ScidFormat::Si5 => {
                let p = Si5Paths::from_index_path(&index_path);
                (p.namebase, p.games)
            }
        };

        if !index_path.exists() {
            return Err(anyhow!("Index file not found: {}", index_path.display()));
        }

        let index_bytes = fs::read(&index_path)
            .with_context(|| format!("Reading {}", index_path.display()))?;
        let names_bytes = fs::read(&namebase_path)
            .with_context(|| format!("Reading {}", namebase_path.display()))?;

        let (entries, names) = match format {
            ScidFormat::Si4 => {
                let header = chess_scid_rw::si4::index::read_header(&index_bytes)
                    .map_err(|e| anyhow!("Failed to read SI4 header: {:?}", e))?;
                let entries = chess_scid_rw::si4::index::read_all_entries(&index_bytes, &header)
                    .map_err(|e| anyhow!("Failed to read SI4 index entries: {:?}", e))?;
                let names = chess_scid_rw::si4::namebase::read_namebase(&names_bytes)
                    .map_err(|e| anyhow!("Failed to read SI4 names: {:?}", e))?;
                (entries, names)
            }
            ScidFormat::Si5 => {
                let entries = chess_scid_rw::si5::index::read_all_entries(&index_bytes)
                    .map_err(|e| anyhow!("Failed to read SI5 index entries: {:?}", e))?;
                let names = chess_scid_rw::si5::namebase::read_namebase(&names_bytes)
                    .map_err(|e| anyhow!("Failed to read SI5 names: {:?}", e))?;
                (entries, names)
            }
        };

        let games_mmap = if games_path.exists() && games_path.metadata()?.len() > 0 {
            let file = File::open(&games_path)?;
            Some(unsafe { Mmap::map(&file)? })
        } else {
            None
        };

        Ok(Self {
            format,
            entries,
            names,
            index_path,
            namebase_path,
            games_path,
            games_mmap,
            pending_games: Vec::new(),
            dirty: false,
            player_ranks: std::sync::OnceLock::new(),
            event_ranks: std::sync::OnceLock::new(),
            site_ranks: std::sync::OnceLock::new(),
            round_ranks: std::sync::OnceLock::new(),
            query_cache: std::sync::Mutex::new(None),
        })
    }

    pub fn create(path: &Path, format: ScidFormat) -> Result<Self> {
        let index_path = match format {
            ScidFormat::Si4 => path.with_extension("si4"),
            ScidFormat::Si5 => path.with_extension("si5"),
        };
        let (namebase_path, games_path) = match format {
            ScidFormat::Si4 => {
                let p = Si4Paths::from_index_path(&index_path);
                (p.namebase, p.games)
            }
            ScidFormat::Si5 => {
                let p = Si5Paths::from_index_path(&index_path);
                (p.namebase, p.games)
            }
        };

        Ok(Self {
            format,
            entries: Vec::new(),
            names: NameTables::default(),
            index_path,
            namebase_path,
            games_path,
            games_mmap: None,
            pending_games: Vec::new(),
            dirty: true,
            player_ranks: std::sync::OnceLock::new(),
            event_ranks: std::sync::OnceLock::new(),
            site_ranks: std::sync::OnceLock::new(),
            round_ranks: std::sync::OnceLock::new(),
            query_cache: std::sync::Mutex::new(None),
        })
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

    pub fn format(&self) -> ScidFormat {
        self.format
    }

    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    pub fn game_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    pub fn names(&self) -> &NameTables {
        &self.names
    }

    pub fn get_blob(&self, entry: &IndexEntry) -> Result<&[u8]> {
        let offset = entry.offset as usize;
        let length = entry.length as usize;

        if let Some(ref mmap) = self.games_mmap {
            if offset + length <= mmap.len() {
                return Ok(&mmap[offset..offset + length]);
            }
            let mmap_len = mmap.len();
            if offset >= mmap_len {
                let pend_off = offset - mmap_len;
                if pend_off + length <= self.pending_games.len() {
                    return Ok(&self.pending_games[pend_off..pend_off + length]);
                }
            }
        } else if offset + length <= self.pending_games.len() {
            return Ok(&self.pending_games[offset..offset + length]);
        }

        Err(anyhow!(
            "Invalid game blob offset {} with length {}",
            offset,
            length
        ))
    }

    pub fn game_pgn(&self, index: usize) -> Result<String> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| anyhow!("Game index {} out of bounds", index))?;

        let blob = self.get_blob(entry)?;
        chess_scid_rw::pgn_build::build_pgn(entry, &self.names, blob)
            .map_err(|e| anyhow!("Error decoding game {}: {:?}", index, e))
    }

    pub fn add_game(&mut self, pgn: &str) -> Result<usize> {
        let parsed = pgn_ingest::parse_game(pgn)
            .map_err(|e| anyhow!("Failed to parse PGN: {:?}", e))?;

        let encoded_blob = chess_scid_rw::game_blob::encode_mainline(&parsed.game)
            .map_err(|e| anyhow!("Failed to encode game blob: {:?}", e))?;

        let current_base_len = self.games_mmap.as_ref().map(|m| m.len()).unwrap_or(0);
        let offset = (current_base_len + self.pending_games.len()) as u64;
        let length = encoded_blob.len() as u32;

        let white_id = self.names.player_id_or_insert(&parsed.tags.white);
        let black_id = self.names.player_id_or_insert(&parsed.tags.black);
        let event_id = self.names.event_id_or_insert(&parsed.tags.event);
        let site_id = self.names.site_id_or_insert(&parsed.tags.site);
        let round_id = self.names.round_id_or_insert(&parsed.tags.round);

        let new_idx = self.entries.len();
        self.entries.push(IndexEntry {
            offset,
            length,
            white_id,
            black_id,
            event_id,
            site_id,
            round_id,
            result: parsed.tags.result,
            eco_code: parsed.tags.eco_code,
            date: parsed.tags.date,
            white_elo: parsed.tags.white_elo,
            black_elo: parsed.tags.black_elo,
            non_standard_start: !parsed.game.is_standard_start(),
            deleted: false,
        });

        self.pending_games.extend_from_slice(&encoded_blob);
        self.dirty = true;
        if let Ok(mut g) = self.query_cache.lock() {
            *g = None;
        }
        Ok(new_idx)
    }

    pub fn update_game(&mut self, index: usize, pgn: &str) -> Result<()> {
        if index >= self.entries.len() {
            return Err(anyhow!("Game index {} out of bounds", index));
        }

        let parsed = pgn_ingest::parse_game(pgn)
            .map_err(|e| anyhow!("Failed to parse PGN: {:?}", e))?;

        let encoded_blob = chess_scid_rw::game_blob::encode_mainline(&parsed.game)
            .map_err(|e| anyhow!("Failed to encode game blob: {:?}", e))?;

        let current_base_len = self.games_mmap.as_ref().map(|m| m.len()).unwrap_or(0);
        let offset = (current_base_len + self.pending_games.len()) as u64;
        let length = encoded_blob.len() as u32;

        let white_id = self.names.player_id_or_insert(&parsed.tags.white);
        let black_id = self.names.player_id_or_insert(&parsed.tags.black);
        let event_id = self.names.event_id_or_insert(&parsed.tags.event);
        let site_id = self.names.site_id_or_insert(&parsed.tags.site);
        let round_id = self.names.round_id_or_insert(&parsed.tags.round);
        let current_deleted = self.entries[index].deleted;

        self.entries[index] = IndexEntry {
            offset,
            length,
            white_id,
            black_id,
            event_id,
            site_id,
            round_id,
            result: parsed.tags.result,
            eco_code: parsed.tags.eco_code,
            date: parsed.tags.date,
            white_elo: parsed.tags.white_elo,
            black_elo: parsed.tags.black_elo,
            non_standard_start: !parsed.game.is_standard_start(),
            deleted: current_deleted,
        };

        self.pending_games.extend_from_slice(&encoded_blob);
        self.dirty = true;
        if let Ok(mut g) = self.query_cache.lock() {
            *g = None;
        }
        Ok(())
    }

    pub fn delete_game(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(anyhow!("Game index {} out of bounds", index));
        }
        self.entries[index].deleted = true;
        self.dirty = true;
        if let Ok(mut g) = self.query_cache.lock() {
            *g = None;
        }
        Ok(())
    }

    pub fn undelete_game(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(anyhow!("Game index {} out of bounds", index));
        }
        self.entries[index].deleted = false;
        self.dirty = true;
        if let Ok(mut g) = self.query_cache.lock() {
            *g = None;
        }
        Ok(())
    }

    pub fn is_deleted(&self, index: usize) -> Option<bool> {
        self.entries.get(index).map(|e| e.deleted)
    }

    pub fn games_path(&self) -> &Path {
        &self.games_path
    }

    pub fn compact(&mut self) -> Result<usize> {
        let mut compacted_entries = Vec::with_capacity(self.entries.len());
        let mut compacted_games = Vec::new();

        let old_total_bytes = self.games_mmap.as_ref().map(|m| m.len()).unwrap_or(0) + self.pending_games.len();

        let old_entries = std::mem::take(&mut self.entries);
        for mut entry in old_entries {
            if !entry.deleted {
                if let Ok(blob) = self.get_blob(&entry) {
                    let new_offset = compacted_games.len() as u64;
                    compacted_games.extend_from_slice(blob);
                    entry.offset = new_offset;
                    compacted_entries.push(entry);
                }
            }
        }

        let reclaimed = old_total_bytes.saturating_sub(compacted_games.len());
        self.entries = compacted_entries;
        self.pending_games = compacted_games;
        self.games_mmap = None;
        self.dirty = true;
        if let Ok(mut g) = self.query_cache.lock() {
            *g = None;
        }

        self.save()?;
        Ok(reclaimed)
    }

    pub fn save(&mut self) -> Result<()> {
        // 1. Write Index
        let index_bytes = match self.format {
            ScidFormat::Si4 => chess_scid_rw::si4::index::write_all_entries(&self.entries),
            ScidFormat::Si5 => chess_scid_rw::si5::index::write_all_entries(&self.entries),
        };
        fs::write(&self.index_path, index_bytes)
            .with_context(|| format!("Writing {}", self.index_path.display()))?;

        // 2. Write Namebase
        let names_bytes = match self.format {
            ScidFormat::Si4 => chess_scid_rw::si4::namebase::write_namebase(&self.names),
            ScidFormat::Si5 => chess_scid_rw::si5::namebase::write_namebase(&self.names),
        };
        fs::write(&self.namebase_path, names_bytes)
            .with_context(|| format!("Writing {}", self.namebase_path.display()))?;

        // 3. Append pending games
        if !self.pending_games.is_empty() || self.games_mmap.is_none() {
            if self.games_mmap.is_none() {
                fs::write(&self.games_path, &self.pending_games)?;
            } else {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.games_path)?;
                file.write_all(&self.pending_games)?;
                file.flush()?;
            }
            self.pending_games.clear();

            // Re-map games file into memory
            let file = File::open(&self.games_path)?;
            self.games_mmap = Some(unsafe { Mmap::map(&file)? });
        }

        self.dirty = false;
        Ok(())
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
        let is_exact_mode = mode_param.map(|m| {
            let m = m.to_lowercase();
            m == "exact" || m == "auto" || m.is_empty()
        }).unwrap_or(true);

        if is_exact_mode && turn_param.is_none() {
            if let Some((_pos, zobrist_hash)) = crate::position_index::parse_target_position(fen_str) {
                if let Ok(pos_idx) = crate::position_index::PositionIndex::load(&self.index_path) {
                    if let Some(gids) = pos_idx.get_all_position_games(zobrist_hash) {
                        let matches: Vec<crate::position_search::PositionMatch> = gids
                            .into_iter()
                            .map(|gid| crate::position_search::PositionMatch {
                                game_id: gid as usize,
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

        let matcher = crate::position_search::parse_position_matcher(fen_str, turn_param, mode_param)?;
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
        crate::position_search::search_material_mmap_with_progress(&self.entries, &self.games_path, filter, progress)
    }

    pub fn search_material(
        &self,
        filter: &crate::position_search::MaterialFilter,
    ) -> Result<Vec<usize>> {
        self.search_material_with_progress(filter, |_, _, _| {})
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
                    self.sort_indices(&mut sorted_indices, filter.sort_by.as_deref(), filter.sort_asc);
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
            self.search_material_with_progress(m, &progress).ok().map(|vec| {
                vec.into_iter().collect::<std::collections::HashSet<usize>>()
            })
        });

        let has_filter = candidate_ids.is_some()
            || mat_matches.is_some()
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
                        let w_ok = (entry.white_id as usize) < m.len() && m[entry.white_id as usize];
                        let b_ok = (entry.black_id as usize) < m.len() && m[entry.black_id as usize];
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
                        let w_ok = (entry.white_id as usize) < m.len() && m[entry.white_id as usize];
                        let b_ok = (entry.black_id as usize) < m.len() && m[entry.black_id as usize];
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
        self.sort_indices(&mut matched_indices, filter.sort_by.as_deref(), filter.sort_asc);

        let total_matches = matched_indices.len();
        let start = page * page_size;

        if let Ok(mut guard) = self.query_cache.lock() {
            *guard = Some((filter.clone(), matched_indices.clone()));
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

    pub fn sort_indices(&self, matched_indices: &mut [usize], sort_by: Option<&str>, sort_asc: Option<bool>) {
        if let Some(sort_field) = sort_by {
            let is_asc = sort_asc.unwrap_or(true);
            let entries = &self.entries;

            match sort_field.to_lowercase().as_str() {
                "date" => {
                    if is_asc {
                        matched_indices.par_sort_unstable_by_key(|&i| entries[i].date);
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| entries[b].date.cmp(&entries[a].date));
                    }
                }
                "white_elo" => {
                    if is_asc {
                        matched_indices.par_sort_unstable_by_key(|&i| entries[i].white_elo);
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| entries[b].white_elo.cmp(&entries[a].white_elo));
                    }
                }
                "black_elo" => {
                    if is_asc {
                        matched_indices.par_sort_unstable_by_key(|&i| entries[i].black_elo);
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| entries[b].black_elo.cmp(&entries[a].black_elo));
                    }
                }
                "eco" => {
                    if is_asc {
                        matched_indices.par_sort_unstable_by_key(|&i| entries[i].eco_code);
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| entries[b].eco_code.cmp(&entries[a].eco_code));
                    }
                }
                "result" => {
                    if is_asc {
                        matched_indices.par_sort_unstable_by_key(|&i| entries[i].result);
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| entries[b].result.cmp(&entries[a].result));
                    }
                }
                "white" => {
                    let ranks = self.get_player_ranks();
                    if is_asc {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].white_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].white_id as usize).copied().unwrap_or(u32::MAX);
                            ra.cmp(&rb)
                        });
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].white_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].white_id as usize).copied().unwrap_or(u32::MAX);
                            rb.cmp(&ra)
                        });
                    }
                }
                "black" => {
                    let ranks = self.get_player_ranks();
                    if is_asc {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].black_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].black_id as usize).copied().unwrap_or(u32::MAX);
                            ra.cmp(&rb)
                        });
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].black_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].black_id as usize).copied().unwrap_or(u32::MAX);
                            rb.cmp(&ra)
                        });
                    }
                }
                "event" => {
                    let ranks = self.get_event_ranks();
                    if is_asc {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].event_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].event_id as usize).copied().unwrap_or(u32::MAX);
                            ra.cmp(&rb)
                        });
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].event_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].event_id as usize).copied().unwrap_or(u32::MAX);
                            rb.cmp(&ra)
                        });
                    }
                }
                "site" => {
                    let ranks = self.get_site_ranks();
                    if is_asc {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].site_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].site_id as usize).copied().unwrap_or(u32::MAX);
                            ra.cmp(&rb)
                        });
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].site_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].site_id as usize).copied().unwrap_or(u32::MAX);
                            rb.cmp(&ra)
                        });
                    }
                }
                "round" => {
                    let ranks = self.get_round_ranks();
                    if is_asc {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].round_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].round_id as usize).copied().unwrap_or(u32::MAX);
                            ra.cmp(&rb)
                        });
                    } else {
                        matched_indices.par_sort_unstable_by(|&a, &b| {
                            let ra = ranks.get(entries[a].round_id as usize).copied().unwrap_or(u32::MAX);
                            let rb = ranks.get(entries[b].round_id as usize).copied().unwrap_or(u32::MAX);
                            rb.cmp(&ra)
                        });
                    }
                }
                "id" | "index" => {
                    if !is_asc {
                        matched_indices.par_sort_unstable_by(|a, b| b.cmp(a));
                    } else {
                        matched_indices.par_sort_unstable();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn stats(&self) -> DbStats {
        let total_games = self.entries.len();
        let deleted_games = self.entries.iter().filter(|e| e.deleted).count();
        let active_games = total_games.saturating_sub(deleted_games);

        let idx_size = self.index_path.metadata().map(|m| m.len()).unwrap_or(0);
        let nb_size = self.namebase_path.metadata().map(|m| m.len()).unwrap_or(0);
        let g_size = self.games_path.metadata().map(|m| m.len()).unwrap_or(0) + self.pending_games.len() as u64;

        DbStats {
            format: self.format,
            index_path: self.index_path.to_string_lossy().to_string(),
            total_games,
            active_games,
            deleted_games,
            players_count: self.names.players.len(),
            events_count: self.names.events.len(),
            sites_count: self.names.sites.len(),
            rounds_count: self.names.rounds.len(),
            index_file_size: idx_size,
            namebase_file_size: nb_size,
            games_file_size: g_size,
        }
    }
}
