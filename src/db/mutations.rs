use anyhow::{anyhow, Context, Result};
use chess_scid_rw::entry::IndexEntry;
use chess_scid_rw::pgn_ingest;
use memmap2::Mmap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;

use super::core::ScidDatabaseWrapper;
use super::types::ScidFormat;

impl ScidDatabaseWrapper {
    pub fn add_game(&mut self, pgn: &str) -> Result<usize> {
        let parsed =
            pgn_ingest::parse_game(pgn).map_err(|e| anyhow!("Failed to parse PGN: {:?}", e))?;

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
        self.clear_query_caches();
        Ok(new_idx)
    }

    pub fn update_game(&mut self, index: usize, pgn: &str) -> Result<()> {
        if index >= self.entries.len() {
            return Err(anyhow!("Game index {} out of bounds", index));
        }

        let parsed =
            pgn_ingest::parse_game(pgn).map_err(|e| anyhow!("Failed to parse PGN: {:?}", e))?;

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
        self.clear_query_caches();
        Ok(())
    }

    pub fn delete_game(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(anyhow!("Game index {} out of bounds", index));
        }
        self.entries[index].deleted = true;
        self.dirty = true;
        self.clear_query_caches();
        Ok(())
    }

    pub fn undelete_game(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(anyhow!("Game index {} out of bounds", index));
        }
        self.entries[index].deleted = false;
        self.dirty = true;
        self.clear_query_caches();
        Ok(())
    }

    pub fn compact(&mut self) -> Result<usize> {
        let mut compacted_entries = Vec::with_capacity(self.entries.len());
        let mut compacted_games = Vec::new();

        let old_total_bytes =
            self.games_mmap.as_ref().map(|m| m.len()).unwrap_or(0) + self.pending_games.len();

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
        self.clear_query_caches();

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
}
