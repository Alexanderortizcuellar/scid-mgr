use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::core::PgnDatabaseWrapper;

// ---------------------------------------------------------------------------
// Multi-Column Sort Engine & Exporter
// ---------------------------------------------------------------------------

impl PgnDatabaseWrapper {
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
                    sort_by_u64!(|idx: usize| entries[idx].eco);
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

    /// Sorts all games in the PGN file according to specified criteria and writes them to a new PGN file.
    pub fn sort_and_export<P: AsRef<Path>>(
        &self,
        output_path: P,
        sort_by: Option<&str>,
        sort_asc: bool,
    ) -> Result<usize> {
        let mut indices: Vec<usize> = (0..self.entries.len()).collect();
        self.sort_indices(&mut indices, sort_by, Some(sort_asc));

        let file = File::create(output_path.as_ref()).with_context(|| {
            format!(
                "Failed to create output PGN file: {}",
                output_path.as_ref().display()
            )
        })?;
        let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);

        for &idx in &indices {
            let entry = &self.entries[idx];
            let start = entry.offset as usize;
            let end = start + entry.length as usize;
            if end <= self.mmap.len() && start < end {
                let slice = &self.mmap[start..end];
                writer.write_all(slice)?;
                if !slice.ends_with(b"\n\n") {
                    if slice.ends_with(b"\n") {
                        writer.write_all(b"\n")?;
                    } else {
                        writer.write_all(b"\n\n")?;
                    }
                }
            }
        }
        writer.flush()?;
        Ok(indices.len())
    }
}

/// Standalone function to sort any PGN file by field (e.g. "date", "white_elo", "white", etc.)
pub fn sort_pgn_file<P1: AsRef<Path>, P2: AsRef<Path>>(
    input_path: P1,
    output_path: P2,
    sort_by: Option<&str>,
    sort_asc: bool,
) -> Result<usize> {
    let db = PgnDatabaseWrapper::open(input_path.as_ref())?;
    db.sort_and_export(output_path, sort_by, sort_asc)
}
