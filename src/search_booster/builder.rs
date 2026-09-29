use anyhow::{Context, Result};
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rayon::prelude::*;
use shakmaty::{Chess, Position};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::db::ScidDatabaseWrapper;

use super::codec::resolve_companion_booster_path;
use super::types::{BoostGameEntry, BoostHeader, BoostMove, GAME_ENTRY_SIZE, HEADER_SIZE};

pub type BoosterProgressCallback = Arc<dyn Fn(usize, usize, usize) + Send + Sync>;

pub struct BoostIndexBuilder;

impl BoostIndexBuilder {
    /// Builds `.boost.idx` for a PGN database
    pub fn build_for_pgn(
        pgn_path: impl AsRef<Path>,
        output_path: Option<PathBuf>,
        progress_cb: Option<BoosterProgressCallback>,
    ) -> Result<(PathBuf, usize, u64, u128)> {
        let start = Instant::now();
        let pgn_ref = pgn_path.as_ref();
        let out = output_path.unwrap_or_else(|| resolve_companion_booster_path(pgn_ref));
        let tmp_out = out.with_extension("boost.idx.tmp");

        let pgn_meta = std::fs::metadata(pgn_ref)?;
        let db_file_size = pgn_meta.len();
        let db_mtime_secs = pgn_meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let file = File::open(pgn_ref)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let offsets = crate::pgn::scan_pgn_game_offsets(&mmap);
        let total_games = offsets.len();

        let progress_counter = AtomicUsize::new(0);

        // Decode games in parallel across CPU threads into vector of BoostMove
        let games_moves: Vec<(Vec<BoostMove>, u8)> = offsets
            .par_iter()
            .map(|&(start_pos, end_pos)| {
                let chunk = &mmap[start_pos..end_pos];
                let mut moves = Vec::with_capacity(80);
                let mut result_code = 0u8;

                let mut reader = BufferedReader::new(chunk);
                struct PgnMoveCollector<'a> {
                    pos: Chess,
                    moves: &'a mut Vec<BoostMove>,
                    result_code: &'a mut u8,
                }

                impl<'a> Visitor for PgnMoveCollector<'a> {
                    type Result = ();

                    fn header(&mut self, key: &[u8], value: pgn_reader::RawHeader<'_>) {
                        if key == b"Result" {
                            let r = value.as_bytes();
                            *self.result_code = match r {
                                b"1-0" => 1,
                                b"0-1" => 2,
                                b"1/2-1/2" => 3,
                                _ => 0,
                            };
                        } else if key == b"FEN" {
                            let fen_str = String::from_utf8_lossy(value.as_bytes());
                            if let Ok(fen) = fen_str.parse::<shakmaty::fen::Fen>() {
                                if let Ok(pos) = fen.into_position(shakmaty::CastlingMode::Chess960)
                                {
                                    self.pos = pos;
                                }
                            }
                        }
                    }

                    fn end_headers(&mut self) -> Skip {
                        Skip(false)
                    }

                    fn san(&mut self, san_plus: SanPlus) {
                        if let Ok(m) = san_plus.san.to_move(&self.pos) {
                            let boost_move = BoostMove::from_shakmaty(&m);
                            self.moves.push(boost_move);
                            self.pos.play_unchecked(&m);
                        }
                    }

                    fn begin_variation(&mut self) -> Skip {
                        Skip(true)
                    }

                    fn end_game(&mut self) -> Self::Result {}
                }

                let mut collector = PgnMoveCollector {
                    pos: Chess::default(),
                    moves: &mut moves,
                    result_code: &mut result_code,
                };
                let _ = reader.read_all(&mut collector);

                let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(ref cb) = progress_cb {
                    if done.is_multiple_of(10000) || done == total_games {
                        cb(done, total_games, 0);
                    }
                }

                (moves, result_code)
            })
            .collect();

        // Write to temporary binary booster file
        let (total_plies, elapsed) = Self::write_booster_file(
            &tmp_out,
            total_games,
            db_mtime_secs,
            db_file_size,
            &games_moves,
            start,
        )?;

        // Atomic rename to final target
        if out.exists() {
            let _ = std::fs::remove_file(&out);
        }
        std::fs::rename(&tmp_out, &out)
            .with_context(|| format!("Failed to rename {:?} to {:?}", tmp_out, out))?;

        Ok((out, total_games, total_plies, elapsed))
    }

    /// Builds `.boost.idx` for a SCID database (.si5 / .si4)
    pub fn build_for_scid(
        scid_db: &ScidDatabaseWrapper,
        output_path: Option<PathBuf>,
        progress_cb: Option<BoosterProgressCallback>,
    ) -> Result<(PathBuf, usize, u64, u128)> {
        let start = Instant::now();
        let games_path_buf = scid_db.games_path().to_path_buf();
        let out = output_path.unwrap_or_else(|| resolve_companion_booster_path(&games_path_buf));
        let tmp_out = out.with_extension("boost.idx.tmp");

        let total_games = scid_db.game_count();
        let entries = scid_db.entries();

        let (db_file_size, db_mtime_secs) =
            if let Ok(meta) = std::fs::metadata(scid_db.games_path()) {
                let sz = meta.len();
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                (sz, mtime)
            } else {
                (0, 0)
            };

        let games_file = File::open(scid_db.games_path())
            .with_context(|| format!("Opening games file {:?}", scid_db.games_path()))?;
        let games_mmap = unsafe { memmap2::Mmap::map(&games_file)? };

        let progress_counter = AtomicUsize::new(0);
        let game_ids: Vec<usize> = (0..total_games).collect();

        let games_moves: Vec<(Vec<BoostMove>, u8)> = game_ids
            .par_chunks(2000)
            .flat_map(|chunk| {
                let mut chunk_res = Vec::with_capacity(chunk.len());

                for &gid in chunk {
                    let entry = &entries[gid];
                    if entry.deleted {
                        chunk_res.push((Vec::new(), entry.result));
                        let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(ref cb) = progress_cb {
                            if done.is_multiple_of(10000) || done == total_games {
                                cb(done, total_games, 0);
                            }
                        }
                        continue;
                    }

                    let start = entry.offset as usize;
                    let end = start + entry.length as usize;
                    if end > games_mmap.len() || start >= end {
                        chunk_res.push((Vec::new(), entry.result));
                        continue;
                    }

                    let blob = &games_mmap[start..end];
                    let mut cursor = 0;

                    let mut pos =
                        match crate::position_search::parse_start_position(blob, &mut cursor) {
                            Some(p) => p,
                            None => {
                                chunk_res.push((Vec::new(), entry.result));
                                continue;
                            }
                        };

                    let mut slots = crate::position_search::standard_piece_slots();
                    let mut counts = [16usize, 16usize];
                    let mut moves = Vec::with_capacity(80);

                    while cursor < blob.len() {
                        let b = blob[cursor];
                        cursor += 1;

                        if b == 15 {
                            break;
                        }
                        if b == 11 {
                            cursor += 1;
                            continue;
                        }
                        if b == 12 || b == 13 || b == 14 {
                            continue;
                        }

                        let (mv, piece_idx, to_sq, is_k, is_q, cap_sq) =
                            match crate::position_search::decode_raw_move(
                                b,
                                &mut cursor,
                                blob,
                                &pos,
                                &slots,
                                &counts,
                            ) {
                                Some(res) => res,
                                None => break,
                            };

                        if !pos.is_legal(&mv) {
                            break;
                        }

                        let boost_move = BoostMove::from_shakmaty(&mv);
                        moves.push(boost_move);

                        let side_idx = usize::from(pos.turn() == shakmaty::Color::Black);
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

                        pos.play_unchecked(&mv);
                    }

                    chunk_res.push((moves, entry.result));
                    let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                    if let Some(ref cb) = progress_cb {
                        if done.is_multiple_of(10000) || done == total_games {
                            cb(done, total_games, 0);
                        }
                    }
                }

                chunk_res
            })
            .collect();

        // Write to temporary binary booster file
        let (total_plies, elapsed) = Self::write_booster_file(
            &tmp_out,
            total_games,
            db_mtime_secs,
            db_file_size,
            &games_moves,
            start,
        )?;

        // Atomic rename to final target
        if out.exists() {
            let _ = std::fs::remove_file(&out);
        }
        std::fs::rename(&tmp_out, &out)
            .with_context(|| format!("Failed to rename {:?} to {:?}", tmp_out, out))?;

        Ok((out, total_games, total_plies, elapsed))
    }

    /// Internal serialization routine writing Header, Directory Table, and Move Payload
    fn write_booster_file(
        path: &Path,
        game_count: usize,
        db_mtime_secs: u64,
        db_file_size: u64,
        games_moves: &[(Vec<BoostMove>, u8)],
        start_time: Instant,
    ) -> Result<(u64, u128)> {
        let file =
            File::create(path).with_context(|| format!("Creating booster file at {:?}", path))?;
        let mut writer = BufWriter::with_capacity(1024 * 1024, file);

        let directory_offset = HEADER_SIZE as u64;
        let payload_offset = directory_offset + (game_count as u64 * GAME_ENTRY_SIZE as u64);

        // 1. Write placeholder header
        let placeholder_header = BoostHeader::new(
            game_count as u32,
            0,
            db_mtime_secs,
            db_file_size,
            directory_offset,
            payload_offset,
        );
        writer.write_all(&placeholder_header.to_bytes())?;

        // 2. Build and write Game Directory Table
        let mut current_move_offset = 0u32;
        let mut total_plies = 0u64;

        for (moves, result) in games_moves {
            let ply_count = moves.len() as u16;
            let entry = BoostGameEntry::new(current_move_offset, ply_count, *result, false);
            writer.write_all(&entry.to_bytes())?;

            current_move_offset += ply_count as u32;
            total_plies += ply_count as u64;
        }

        // 3. Write Move Payload Buffer
        for (moves, _) in games_moves {
            for m in moves {
                writer.write_all(&m.0.to_le_bytes())?;
            }
        }

        writer.flush()?;
        drop(writer);

        // 4. Update Header with true total_plies
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        let final_header = BoostHeader::new(
            game_count as u32,
            total_plies,
            db_mtime_secs,
            db_file_size,
            directory_offset,
            payload_offset,
        );
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&final_header.to_bytes())?;
        file.flush()?;

        let elapsed = start_time.elapsed().as_millis();
        Ok((total_plies, elapsed))
    }
}
