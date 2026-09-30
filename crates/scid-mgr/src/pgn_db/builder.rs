use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use super::types::{
    pack_date, pack_eco, pack_result, CompactPgnRecord, PgnIndexHeader, PgnNameTables,
    RawGameRecord, PGN_INDEX_MAGIC, PGN_INDEX_VERSION,
};

/// Serializes scanned game records and deduplicated string tables directly to a `.pgn.idx` binary file
pub(crate) fn save_index_file(
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
pub(crate) fn scan_pgn_parallel(
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

    let chunk_ranges: Vec<(usize, usize)> = chunk_starts.windows(2).map(|w| (w[0], w[1])).collect();

    let chunk_results: Result<Vec<Vec<RawGameRecord>>> = chunk_ranges
        .into_par_iter()
        .map(|(start_idx, end_idx)| scan_chunk(mmap, start_idx, end_idx))
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

fn scan_chunk<'a>(
    mmap: &'a [u8],
    chunk_start: usize,
    chunk_end: usize,
) -> Result<Vec<RawGameRecord<'a>>> {
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
                    if let Some((tag_name, tag_val)) = parse_tag(trimmed) {
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
                while tag_check < mmap.len() && mmap[tag_check] != b'\n' && mmap[tag_check] != b']'
                {
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
