use anyhow::{anyhow, Context, Result};
use chess_scid_rw::entry::IndexEntry;
use chess_scid_rw::names::NameTables;
use chess_scid_rw::{Si4Paths, Si5Paths};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::{DbStats, GameFilter, ScidFormat};

pub struct ScidDatabaseWrapper {
    pub(crate) format: ScidFormat,
    pub(crate) entries: Vec<IndexEntry>,
    pub(crate) names: NameTables,
    pub(crate) index_path: PathBuf,
    pub(crate) namebase_path: PathBuf,
    pub(crate) games_path: PathBuf,
    pub(crate) games_mmap: Option<Mmap>,
    pub(crate) pending_games: Vec<u8>,
    pub(crate) dirty: bool,
    pub(crate) player_ranks: std::sync::OnceLock<Vec<u32>>,
    pub(crate) event_ranks: std::sync::OnceLock<Vec<u32>>,
    pub(crate) site_ranks: std::sync::OnceLock<Vec<u32>>,
    pub(crate) round_ranks: std::sync::OnceLock<Vec<u32>>,
    pub(crate) query_cache: std::sync::Mutex<Option<(GameFilter, Vec<usize>)>>,
    pub(crate) column_sort_cache: std::sync::Mutex<HashMap<String, Arc<Vec<usize>>>>,
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

        let index_bytes =
            fs::read(&index_path).with_context(|| format!("Reading {}", index_path.display()))?;
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
            column_sort_cache: std::sync::Mutex::new(HashMap::new()),
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
            column_sort_cache: std::sync::Mutex::new(HashMap::new()),
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

    pub fn games_path(&self) -> &Path {
        &self.games_path
    }

    pub fn is_deleted(&self, index: usize) -> Option<bool> {
        self.entries.get(index).map(|e| e.deleted)
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

    pub fn stats(&self) -> DbStats {
        let total_games = self.entries.len();
        let deleted_games = self.entries.iter().filter(|e| e.deleted).count();
        let active_games = total_games.saturating_sub(deleted_games);

        let idx_size = self.index_path.metadata().map(|m| m.len()).unwrap_or(0);
        let nb_size = self.namebase_path.metadata().map(|m| m.len()).unwrap_or(0);
        let g_size = self.games_path.metadata().map(|m| m.len()).unwrap_or(0)
            + self.pending_games.len() as u64;

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
