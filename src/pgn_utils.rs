use crate::db::{ScidDatabaseWrapper, ScidFormat};
use anyhow::{Context, Result};
use chess_scid_rw::names::NameTables;
use serde::{Deserialize, Serialize};
use shakmaty::Position;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub imported_games: usize,
    pub errors: usize,
    pub speed_gps: f64,
    pub eta_seconds: u64,
}

pub struct FastNameTables {
    pub players: Vec<String>,
    pub events: Vec<String>,
    pub sites: Vec<String>,
    pub rounds: Vec<String>,
    player_map: HashMap<String, u32>,
    event_map: HashMap<String, u32>,
    site_map: HashMap<String, u32>,
    round_map: HashMap<String, u32>,
}

impl FastNameTables {
    pub fn from_name_tables(tables: &NameTables) -> Self {
        let mut player_map = HashMap::with_capacity(tables.players.len() * 2);
        for (i, p) in tables.players.iter().enumerate() {
            player_map.insert(p.clone(), i as u32);
        }

        let mut event_map = HashMap::with_capacity(tables.events.len() * 2);
        for (i, e) in tables.events.iter().enumerate() {
            event_map.insert(e.clone(), i as u32);
        }

        let mut site_map = HashMap::with_capacity(tables.sites.len() * 2);
        for (i, s) in tables.sites.iter().enumerate() {
            site_map.insert(s.clone(), i as u32);
        }

        let mut round_map = HashMap::with_capacity(tables.rounds.len() * 2);
        for (i, r) in tables.rounds.iter().enumerate() {
            round_map.insert(r.clone(), i as u32);
        }

        Self {
            players: tables.players.clone(),
            events: tables.events.clone(),
            sites: tables.sites.clone(),
            rounds: tables.rounds.clone(),
            player_map,
            event_map,
            site_map,
            round_map,
        }
    }

    #[inline]
    pub fn player_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.player_map.get(name) {
            return id;
        }
        let id = self.players.len() as u32;
        let owned = name.to_string();
        self.players.push(owned.clone());
        self.player_map.insert(owned, id);
        id
    }

    #[inline]
    pub fn event_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.event_map.get(name) {
            return id;
        }
        let id = self.events.len() as u32;
        let owned = name.to_string();
        self.events.push(owned.clone());
        self.event_map.insert(owned, id);
        id
    }

    #[inline]
    pub fn site_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.site_map.get(name) {
            return id;
        }
        let id = self.sites.len() as u32;
        let owned = name.to_string();
        self.sites.push(owned.clone());
        self.site_map.insert(owned, id);
        id
    }

    #[inline]
    pub fn round_id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.round_map.get(name) {
            return id;
        }
        let id = self.rounds.len() as u32;
        let owned = name.to_string();
        self.rounds.push(owned.clone());
        self.round_map.insert(owned, id);
        id
    }

    pub fn to_name_tables(&self) -> NameTables {
        NameTables {
            players: self.players.clone(),
            events: self.events.clone(),
            sites: self.sites.clone(),
            rounds: self.rounds.clone(),
        }
    }
}

pub fn import_pgn_file_with_progress<F>(
    db: &mut ScidDatabaseWrapper,
    pgn_path: &Path,
    progress_cb: F,
) -> Result<(usize, usize)>
where
    F: FnMut(&ImportProgress),
{
    let format = db.format();
    let index_path = db.index_path().to_path_buf();

    let res = crate::zero_copy_ingest::import_pgn_ultra_fast(
        &index_path,
        pgn_path,
        format,
        progress_cb,
    )?;

    *db = ScidDatabaseWrapper::open(&index_path)?;
    Ok(res)
}

#[allow(dead_code)]
pub fn import_pgn_with_scid_cli(
    db: &mut ScidDatabaseWrapper,
    pgn_path: &Path,
    scid_exe_path: &Path,
) -> Result<(usize, usize)> {
    let index_path = db.index_path().to_path_buf();
    let codec = match db.format() {
        ScidFormat::Si4 => "SCID4",
        ScidFormat::Si5 => "SCID5",
    };

    let scid_dir = scid_exe_path.parent().unwrap_or(Path::new("."));
    let scripts_dir = scid_dir.join("scripts");
    let fast_import_script = scripts_dir.join("fast_import.tcl");

    if !fast_import_script.exists() {
        let _ = fs::create_dir_all(&scripts_dir);
        let tcl_content = r#"lassign $argv db_basename pgn_file codec
if {$db_basename eq "" || $pgn_file eq ""} { exit 1 }
if {$codec eq ""} { set codec "SCID5" }
if {[catch {set db [sc_base open $codec $db_basename]} err]} {
    if {[catch {set db [sc_base create $codec $db_basename]} err]} { exit 1 }
}
if {[catch {set res [sc_base import $db $pgn_file]} err]} { sc_base close $db; exit 1 }
set num_imported [lindex $res 0]
puts "IMPORTED: $num_imported"
sc_base close $db
exit 0
"#;
        let _ = fs::write(&fast_import_script, tcl_content);
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    let abs_index = if index_path.is_relative() {
        cwd.join(&index_path)
    } else {
        index_path.clone()
    };
    let abs_basename = abs_index.with_extension("");
    let abs_basename_str = abs_basename.to_string_lossy().replace('\\', "/");

    let abs_pgn = if pgn_path.is_relative() {
        cwd.join(pgn_path)
    } else {
        pgn_path.to_path_buf()
    };
    let abs_pgn_str = abs_pgn.to_string_lossy().replace('\\', "/");

    let status = std::process::Command::new(scid_exe_path)
        .arg("scripts/fast_import.tcl")
        .arg(&abs_basename_str)
        .arg(&abs_pgn_str)
        .arg(codec)
        .current_dir(scid_dir)
        .status()
        .context("Failed to run scid.exe C++ engine")?;

    if !status.success() {
        return Err(anyhow::anyhow!("scid.exe failed with exit status: {:?}", status));
    }

    *db = ScidDatabaseWrapper::open(&index_path)?;
    Ok((db.game_count(), 0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    pub exported_games: usize,
    pub total_games: usize,
    pub percent: f64,
    pub speed_gps: f64,
    pub eta_seconds: u64,
}

pub fn export_pgn_file(db: &ScidDatabaseWrapper, output_path: &Path) -> Result<usize> {
    export_pgn_ultra_fast(db, output_path, |_| {})
}

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

        let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) = match crate::position_search::decode_raw_move(
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
    use rayon::prelude::*;
    use std::io::BufWriter;
    use std::time::Instant;

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
