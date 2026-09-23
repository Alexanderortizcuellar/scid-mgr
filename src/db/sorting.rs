use anyhow::{Context, Result};
use chess_scid_rw::{Si4Paths, Si5Paths};
use rayon::prelude::*;
use std::fs;
use std::path::Path;

use super::core::{detect_format_from_path, ScidDatabaseWrapper};
use super::types::ScidFormat;

impl ScidDatabaseWrapper {
    /// Sorts the database in-place, rewriting and compacting the move streams and index entries in the sorted order.
    pub fn sort_database(
        &mut self,
        sort_by: &str,
        sort_asc: bool,
        delete_removed: bool,
    ) -> Result<usize> {
        let mut indices: Vec<usize> = if delete_removed {
            (0..self.entries.len())
                .filter(|&i| !self.entries[i].deleted)
                .collect()
        } else {
            (0..self.entries.len()).collect()
        };

        self.sort_indices(&mut indices, Some(sort_by), Some(sort_asc));

        let mut sorted_entries = Vec::with_capacity(indices.len());
        let mut sorted_games = Vec::new();

        for idx in indices {
            let mut entry = self.entries[idx].clone();
            if let Ok(blob) = self.get_blob(&entry) {
                let new_offset = sorted_games.len() as u64;
                sorted_games.extend_from_slice(blob);
                entry.offset = new_offset;
                sorted_entries.push(entry);
            }
        }

        let count = sorted_entries.len();
        self.entries = sorted_entries;
        self.pending_games = sorted_games;
        self.games_mmap = None;
        self.dirty = true;
        self.clear_query_caches();

        // Save rewritten index, namebase, and games
        self.save()?;

        // If a companion .pos.idx existed, remove it as Game IDs have been reordered
        let pos_idx_path = crate::position_index::PositionIndex::companion_path(&self.index_path);
        if pos_idx_path.exists() {
            let _ = std::fs::remove_file(&pos_idx_path);
        }

        Ok(count)
    }

    /// Sorts the database and writes the resulting sorted database to a new destination path.
    pub fn sort_database_to(
        &self,
        dest_path: &Path,
        sort_by: &str,
        sort_asc: bool,
        delete_removed: bool,
    ) -> Result<usize> {
        let (dest_format, dest_index_path) = detect_format_from_path(dest_path);
        let (dest_namebase_path, dest_games_path) = match dest_format {
            ScidFormat::Si4 => {
                let p = Si4Paths::from_index_path(&dest_index_path);
                (p.namebase, p.games)
            }
            ScidFormat::Si5 => {
                let p = Si5Paths::from_index_path(&dest_index_path);
                (p.namebase, p.games)
            }
        };

        let mut indices: Vec<usize> = if delete_removed {
            (0..self.entries.len())
                .filter(|&i| !self.entries[i].deleted)
                .collect()
        } else {
            (0..self.entries.len()).collect()
        };

        self.sort_indices(&mut indices, Some(sort_by), Some(sort_asc));

        let mut sorted_entries = Vec::with_capacity(indices.len());
        let mut sorted_games = Vec::new();

        for idx in indices {
            let mut entry = self.entries[idx].clone();
            if let Ok(blob) = self.get_blob(&entry) {
                let new_offset = sorted_games.len() as u64;
                sorted_games.extend_from_slice(blob);
                entry.offset = new_offset;
                sorted_entries.push(entry);
            }
        }

        let count = sorted_entries.len();

        // 1. Write Index
        let index_bytes = match dest_format {
            ScidFormat::Si4 => chess_scid_rw::si4::index::write_all_entries(&sorted_entries),
            ScidFormat::Si5 => chess_scid_rw::si5::index::write_all_entries(&sorted_entries),
        };
        fs::write(&dest_index_path, index_bytes)
            .with_context(|| format!("Writing {}", dest_index_path.display()))?;

        // 2. Write Namebase
        let names_bytes = match dest_format {
            ScidFormat::Si4 => chess_scid_rw::si4::namebase::write_namebase(&self.names),
            ScidFormat::Si5 => chess_scid_rw::si5::namebase::write_namebase(&self.names),
        };
        fs::write(&dest_namebase_path, names_bytes)
            .with_context(|| format!("Writing {}", dest_namebase_path.display()))?;

        // 3. Write Games
        fs::write(&dest_games_path, &sorted_games)
            .with_context(|| format!("Writing {}", dest_games_path.display()))?;

        Ok(count)
    }

    pub fn sort_indices(
        &self,
        matched_indices: &mut [usize],
        sort_by: Option<&str>,
        sort_asc: Option<bool>,
    ) {
        if let Some(sort_field) = sort_by {
            let is_asc = sort_asc.unwrap_or(true);
            let entries = &self.entries;

            macro_rules! sort_by_u64 {
                ($key_fn:expr) => {{
                    let mut pairs: Vec<u64> = matched_indices
                        .par_iter()
                        .map(|&idx| {
                            let key = ($key_fn(idx)) as u64;
                            (key << 32) | (idx as u64)
                        })
                        .collect();
                    pairs.par_sort_unstable();
                    if !is_asc {
                        pairs.reverse();
                    }
                    for (slot, pair) in matched_indices.iter_mut().zip(pairs.into_iter()) {
                        *slot = (pair & 0xFFFF_FFFF) as usize;
                    }
                }};
            }

            match sort_field.to_lowercase().as_str() {
                "date" => {
                    sort_by_u64!(|idx: usize| entries[idx].date);
                }
                "white_elo" => {
                    sort_by_u64!(|idx: usize| entries[idx].white_elo);
                }
                "black_elo" => {
                    sort_by_u64!(|idx: usize| entries[idx].black_elo);
                }
                "eco" => {
                    sort_by_u64!(|idx: usize| entries[idx].eco_code);
                }
                "result" => {
                    sort_by_u64!(|idx: usize| entries[idx].result);
                }
                "white" => {
                    let ranks = self.get_player_ranks();
                    sort_by_u64!(|idx: usize| {
                        ranks
                            .get(entries[idx].white_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                }
                "black" => {
                    let ranks = self.get_player_ranks();
                    sort_by_u64!(|idx: usize| {
                        ranks
                            .get(entries[idx].black_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                }
                "event" => {
                    let ranks = self.get_event_ranks();
                    sort_by_u64!(|idx: usize| {
                        ranks
                            .get(entries[idx].event_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                }
                "site" => {
                    let ranks = self.get_site_ranks();
                    sort_by_u64!(|idx: usize| {
                        ranks
                            .get(entries[idx].site_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
                }
                "round" => {
                    let ranks = self.get_round_ranks();
                    sort_by_u64!(|idx: usize| {
                        ranks
                            .get(entries[idx].round_id as usize)
                            .copied()
                            .unwrap_or(u32::MAX)
                    });
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
}
