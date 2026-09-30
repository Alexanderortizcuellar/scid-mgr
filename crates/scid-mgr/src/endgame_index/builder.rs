use anyhow::{Context, Result};
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use rayon::prelude::*;
use shakmaty::{Chess, Position};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::db::ScidDatabaseWrapper;
use crate::endgame_index::catalog::EndgameCatalog;
use crate::endgame_index::detector::{EndgameDetector, FeatureDetector};
use crate::endgame_index::model::GameFeatureRecord;
use crate::endgame_index::serializer::FeatureIndexWriter;

pub type EndgameProgressCallback = Arc<dyn Fn(usize, usize, f64) + Send + Sync>;

pub struct EndgameIndexBuilder {
    catalog: EndgameCatalog,
}

impl EndgameIndexBuilder {
    pub fn new() -> Self {
        Self {
            catalog: EndgameCatalog::default_catalog(),
        }
    }

    pub fn with_catalog(catalog: EndgameCatalog) -> Self {
        Self { catalog }
    }

    pub fn build_for_pgn(
        &self,
        pgn_path: impl AsRef<Path>,
        output_path: Option<PathBuf>,
        progress_cb: Option<EndgameProgressCallback>,
    ) -> Result<(PathBuf, usize, u128)> {
        let start = Instant::now();
        let pgn_ref = pgn_path.as_ref();
        let out = output_path.unwrap_or_else(|| pgn_ref.with_extension("feat.idx"));
        let tmp_out = out.with_extension("feat.idx.tmp");

        let pgn_meta = std::fs::metadata(pgn_ref)?;
        let db_file_size = pgn_meta.len();
        let db_mtime_secs = pgn_meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Scan PGN offsets for parallel indexing
        let file = File::open(pgn_ref)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let offsets = crate::pgn::scan_pgn_game_offsets(&mmap);
        let total_games = offsets.len();

        let progress_counter = AtomicUsize::new(0);

        // Process games in parallel chunks
        let records: Vec<GameFeatureRecord> = offsets
            .par_iter()
            .map(|&(start_pos, end_pos)| {
                let chunk = &mmap[start_pos..end_pos];
                let mut detector = EndgameDetector::with_catalog(self.catalog.clone());
                let mut board = Chess::default();

                let mut reader = BufferedReader::new(chunk);
                struct PgnMoveCollector<'a> {
                    board: &'a mut Chess,
                    detector: &'a mut EndgameDetector,
                }
                impl<'a> Visitor for PgnMoveCollector<'a> {
                    type Result = ();
                    fn header(&mut self, key: &[u8], value: pgn_reader::RawHeader<'_>) {
                        if key == b"FEN" {
                            let fen_str = String::from_utf8_lossy(value.as_bytes());
                            if let Ok(fen) = fen_str.parse::<shakmaty::fen::Fen>() {
                                if let Ok(pos) = fen.into_position(shakmaty::CastlingMode::Chess960)
                                {
                                    *self.board = pos;
                                }
                            }
                        }
                    }
                    fn end_headers(&mut self) -> Skip {
                        self.detector.process_position(self.board);
                        Skip(false)
                    }
                    fn san(&mut self, san_plus: SanPlus) {
                        if let Ok(m) = san_plus.san.to_move(self.board) {
                            self.board.play_unchecked(&m);
                            self.detector.process_position(self.board);
                        }
                    }
                    fn begin_variation(&mut self) -> Skip {
                        Skip(true)
                    }
                    fn end_game(&mut self) -> Self::Result {}
                }

                let mut visitor = PgnMoveCollector {
                    board: &mut board,
                    detector: &mut detector,
                };
                let _ = reader.read_game(&mut visitor);

                let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(ref cb) = progress_cb {
                    if done.is_multiple_of(2000) || done == total_games {
                        let pct = (done as f64 / total_games.max(1) as f64) * 100.0;
                        cb(done, total_games, pct);
                    }
                }

                detector.finish_game()
            })
            .collect();

        // Write to temporary feature index
        let mut writer = FeatureIndexWriter::create(
            &tmp_out,
            self.catalog.version,
            self.catalog.features.len() as u16,
            db_mtime_secs,
            db_file_size,
        )?;

        writer.write_records_slice(&records)?;
        writer.finish()?;

        // Atomic rename
        if out.exists() {
            let _ = std::fs::remove_file(&out);
        }
        std::fs::rename(&tmp_out, &out)
            .with_context(|| format!("Failed to rename {:?} to {:?}", tmp_out, out))?;

        let elapsed = start.elapsed().as_millis();
        Ok((out, total_games, elapsed))
    }

    pub fn build_for_scid(
        &self,
        scid_db: &ScidDatabaseWrapper,
        output_path: Option<PathBuf>,
        progress_cb: Option<EndgameProgressCallback>,
    ) -> Result<(PathBuf, usize, u128)> {
        let start = Instant::now();
        let games_path_buf = scid_db.games_path().to_path_buf();
        let out = output_path.unwrap_or_else(|| games_path_buf.with_extension("feat.idx"));
        let tmp_out = out.with_extension("feat.idx.tmp");

        let total_games = scid_db.game_count();
        let entries = scid_db.entries();

        // Gather database file stats for header validation
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

        // Memory-map the games file for blob access
        let games_file = File::open(scid_db.games_path())
            .with_context(|| format!("Opening games file {:?}", scid_db.games_path()))?;
        let games_mmap = unsafe { memmap2::Mmap::map(&games_file)? };

        let progress_counter = AtomicUsize::new(0);

        let game_ids: Vec<usize> = (0..total_games).collect();

        // Process games using the same blob-decoding pattern as continuation_index::dynamic
        let records: Vec<GameFeatureRecord> = game_ids
            .par_chunks(2000)
            .flat_map(|chunk| {
                let mut chunk_records = Vec::with_capacity(chunk.len());
                let mut detector = EndgameDetector::with_catalog(self.catalog.clone());

                for &gid in chunk {
                    detector.reset();

                    let entry = &entries[gid];
                    if entry.deleted {
                        chunk_records.push(detector.finish_game());
                        let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(ref cb) = progress_cb {
                            if done.is_multiple_of(5000) || done == total_games {
                                let pct = (done as f64 / total_games.max(1) as f64) * 100.0;
                                cb(done, total_games, pct);
                            }
                        }
                        continue;
                    }

                    let blob_start = entry.offset as usize;
                    let blob_end = blob_start + entry.length as usize;
                    if blob_end > games_mmap.len() || blob_start >= blob_end {
                        chunk_records.push(detector.finish_game());
                        let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(ref cb) = progress_cb {
                            if done.is_multiple_of(5000) || done == total_games {
                                let pct = (done as f64 / total_games.max(1) as f64) * 100.0;
                                cb(done, total_games, pct);
                            }
                        }
                        continue;
                    }

                    let blob = &games_mmap[blob_start..blob_end];
                    let mut cursor = 0;

                    // Parse start position (handles non-standard start FENs)
                    let mut pos =
                        match crate::position_search::parse_start_position(blob, &mut cursor) {
                            Some(p) => p,
                            None => {
                                chunk_records.push(detector.finish_game());
                                let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                                if let Some(ref cb) = progress_cb {
                                    if done.is_multiple_of(5000) || done == total_games {
                                        let pct = (done as f64 / total_games.max(1) as f64) * 100.0;
                                        cb(done, total_games, pct);
                                    }
                                }
                                continue;
                            }
                        };

                    let mut slots = crate::position_search::standard_piece_slots();
                    let mut counts = [16usize, 16usize];

                    // Evaluate initial position
                    detector.process_position(&pos);

                    // Decode moves from the blob byte-by-byte
                    while cursor < blob.len() {
                        let b = blob[cursor];
                        cursor += 1;

                        // End-of-game marker
                        if b == crate::position_search::ENCODE_END_GAME {
                            break;
                        }
                        // Skip special markers (comments, NAGs, etc.)
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
                        detector.process_position(&pos);
                    }

                    chunk_records.push(detector.finish_game());

                    let done = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                    if let Some(ref cb) = progress_cb {
                        if done.is_multiple_of(5000) || done == total_games {
                            let pct = (done as f64 / total_games.max(1) as f64) * 100.0;
                            cb(done, total_games, pct);
                        }
                    }
                }
                chunk_records
            })
            .collect();

        // Write to temporary feature index
        let mut writer = FeatureIndexWriter::create(
            &tmp_out,
            self.catalog.version,
            self.catalog.features.len() as u16,
            db_mtime_secs,
            db_file_size,
        )?;

        writer.write_records_slice(&records)?;
        writer.finish()?;

        // Atomic rename
        if out.exists() {
            let _ = std::fs::remove_file(&out);
        }
        std::fs::rename(&tmp_out, &out)
            .with_context(|| format!("Failed to rename {:?} to {:?}", tmp_out, out))?;

        let elapsed = start.elapsed().as_millis();
        Ok((out, total_games, elapsed))
    }
}

impl Default for EndgameIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}
