use crate::db::ScidDatabaseWrapper;
use anyhow::{Context, Result};
use chess_scid_rw::names::NameTables;
use rayon::prelude::*;
use shakmaty::Position;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use super::types::ExportProgress;

pub fn fast_game_to_pgn(
    entry: &chess_scid_rw::entry::IndexEntry,
    names: &NameTables,
    blob: &[u8],
) -> Option<String> {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(512);

    let event = names.event(entry.event_id);
    let site = names.site(entry.site_id);
    let date = chess_scid_rw::dates::date_to_pgn(entry.date);
    let round = names.round(entry.round_id);
    let white = names.player(entry.white_id);
    let black = names.player(entry.black_id);
    let result = crate::db::result_code_to_str(entry.result);

    let _ = writeln!(out, "[Event \"{}\"]", event);
    let _ = writeln!(out, "[Site \"{}\"]", site);
    let _ = writeln!(out, "[Date \"{}\"]", date);
    let _ = writeln!(out, "[Round \"{}\"]", round);
    let _ = writeln!(out, "[White \"{}\"]", white);
    let _ = writeln!(out, "[Black \"{}\"]", black);
    let _ = writeln!(out, "[Result \"{}\"]", result);

    if let Some(eco) = chess_scid_rw::eco::eco_to_string(entry.eco_code) {
        let _ = writeln!(out, "[ECO \"{}\"]", eco);
    }
    if entry.white_elo > 0 {
        let _ = writeln!(out, "[WhiteElo \"{}\"]", entry.white_elo);
    }
    if entry.black_elo > 0 {
        let _ = writeln!(out, "[BlackElo \"{}\"]", entry.black_elo);
    }

    // Move stream decode
    let mut cursor = 0;
    let mut pos = match crate::position_search::parse_start_position(blob, &mut cursor) {
        Some(p) => p,
        None => {
            let _ = write!(out, "\n{}\n", result);
            return Some(out);
        }
    };
    if entry.non_standard_start {
        let _ = writeln!(out, "[SetUp \"1\"]");
        let _ = writeln!(out, "[FEN \"{:?}\"]", pos);
    }

    out.push('\n');

    let mut slots = crate::position_search::standard_piece_slots();
    let mut counts = [16usize, 16];
    let mut move_num = 1;
    let mut line_len = 0;

    while cursor < blob.len() {
        let byte = blob[cursor];
        cursor += 1;
        if byte == 15 {
            break;
        }
        if byte == 11 {
            cursor += 1;
            continue;
        }
        if byte == 12 || byte == 13 || byte == 14 {
            continue;
        }

        let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
            match crate::position_search::decode_raw_move(
                byte,
                &mut cursor,
                blob,
                &pos,
                &slots,
                &counts,
            ) {
                Some(m) => m,
                None => break,
            };

        let is_white = pos.turn() == shakmaty::Color::White;
        let san = shakmaty::san::SanPlus::from_move_and_play_unchecked(&mut pos, &mv);

        let move_text = if is_white {
            format!("{}. {} ", move_num, san)
        } else {
            move_num += 1;
            format!("{} ", san)
        };

        if line_len + move_text.len() > 78 {
            out.push('\n');
            line_len = 0;
        }
        out.push_str(&move_text);
        line_len += move_text.len();

        let side_idx = usize::from(!is_white);
        crate::position_search::update_slots_on_move(
            &mut slots,
            &mut counts,
            side_idx,
            piece_idx,
            to_sq,
            is_k,
            is_q,
            cap_sq,
        );
    }

    if line_len > 0 {
        out.push(' ');
    }
    out.push_str(result);
    out.push('\n');
    Some(out)
}

pub fn export_pgn_ultra_fast<F>(
    db: &ScidDatabaseWrapper,
    output_path: &Path,
    mut progress_cb: F,
) -> Result<usize>
where
    F: FnMut(&ExportProgress),
{
    let file = File::create(output_path)
        .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file); // 8MB high-throughput buffer

    let entries = db.entries();
    let names = db.names();
    let total_games = entries.len();

    let start_time = Instant::now();
    let mut last_report = Instant::now();
    let mut exported = 0;

    const CHUNK_SIZE: usize = 8000;
    let num_chunks = total_games.div_ceil(CHUNK_SIZE);

    for chunk_idx in 0..num_chunks {
        let start_i = chunk_idx * CHUNK_SIZE;
        let end_i = (start_i + CHUNK_SIZE).min(total_games);

        // Parallel decode games in chunk
        let chunk_pgns: Vec<Option<String>> = (start_i..end_i)
            .into_par_iter()
            .map(|i| {
                let entry = &entries[i];
                if entry.deleted {
                    return None;
                }
                let blob = db.get_blob(entry).ok()?;
                fast_game_to_pgn(entry, names, blob)
            })
            .collect();

        // High-speed sequential write
        for pgn in chunk_pgns.into_iter().flatten() {
            writer.write_all(pgn.trim().as_bytes())?;
            writer.write_all(b"\n\n")?;
            exported += 1;
        }

        if last_report.elapsed().as_millis() >= 100 || chunk_idx + 1 == num_chunks {
            let total_elapsed = start_time.elapsed().as_secs_f64();
            let speed_gps = if total_elapsed > 0.01 {
                end_i as f64 / total_elapsed
            } else {
                0.0
            };
            let percent = if total_games > 0 {
                (end_i as f64 / total_games as f64) * 100.0
            } else {
                100.0
            };
            let remaining = total_games.saturating_sub(end_i);
            let eta_seconds = if speed_gps > 10.0 {
                (remaining as f64 / speed_gps) as u64
            } else {
                0
            };

            progress_cb(&ExportProgress {
                exported_games: exported,
                total_games,
                percent,
                speed_gps,
                eta_seconds,
            });
            last_report = Instant::now();
        }
    }

    writer.flush()?;
    Ok(exported)
}

pub fn export_pgn_file(db: &ScidDatabaseWrapper, output_path: &Path) -> Result<usize> {
    export_pgn_ultra_fast(db, output_path, |_| {})
}
