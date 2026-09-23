use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};
use rayon::prelude::*;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, EnPassantMode, Position};

use super::codec::encode_posting_payload;
use super::core::PositionIndex;
use super::types::{
    PositionIndexHeader, PositionPostingList, SortedIndexEntry, HEADER_SIZE, INLINE_FLAG,
    NUM_STRIPES, POS_INDEX_MAGIC, POS_INDEX_VERSION,
};

// ---------------------------------------------------------------------------
// Thread-Safe Striped Map for Parallel Indexing
// ---------------------------------------------------------------------------

pub(crate) struct StripedPositionPostingMap {
    stripes: Vec<Mutex<HashMap<u64, PositionPostingList>>>,
    unique_counter: AtomicUsize,
}

impl StripedPositionPostingMap {
    pub(crate) fn new() -> Self {
        let mut stripes = Vec::with_capacity(NUM_STRIPES);
        for _ in 0..NUM_STRIPES {
            stripes.push(Mutex::new(HashMap::with_capacity(1024)));
        }
        Self {
            stripes,
            unique_counter: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn stripe_index(hash: u64) -> usize {
        (hash as usize) % NUM_STRIPES
    }

    pub(crate) fn record(&self, hash: u64, game_id: u32) {
        let idx = Self::stripe_index(hash);
        let mut guard = self.stripes[idx].lock().unwrap();
        use std::collections::hash_map::Entry;
        match guard.entry(hash) {
            Entry::Occupied(mut occ) => {
                occ.get_mut().add(game_id);
            }
            Entry::Vacant(vac) => {
                let mut posting = PositionPostingList::new(hash);
                posting.add(game_id);
                vac.insert(posting);
                self.unique_counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    #[inline]
    pub fn total_positions(&self) -> usize {
        self.unique_counter.load(Ordering::Relaxed)
    }

    pub(crate) fn into_map(self) -> HashMap<u64, PositionPostingList> {
        let total_size: usize = self.stripes.iter().map(|s| s.lock().unwrap().len()).sum();
        let mut combined: HashMap<u64, PositionPostingList> = HashMap::with_capacity(total_size);
        for stripe in self.stripes {
            let map = stripe.into_inner().unwrap();
            for (k, v) in map {
                if let Some(existing) = combined.get_mut(&k) {
                    existing.merge(v);
                } else {
                    combined.insert(k, v);
                }
            }
        }
        combined
    }
}

// ---------------------------------------------------------------------------
// PGN Position Visitor
// ---------------------------------------------------------------------------

pub(crate) struct PgnPositionVisitor<'a> {
    max_ply: usize,
    game_id: u32,
    accumulator: &'a StripedPositionPostingMap,
    pos: Chess,
    ply: usize,
}

impl<'a> PgnPositionVisitor<'a> {
    pub(crate) fn new(
        max_ply: usize,
        game_id: u32,
        accumulator: &'a StripedPositionPostingMap,
    ) -> Self {
        Self {
            max_ply,
            game_id,
            accumulator,
            pos: Chess::default(),
            ply: 0,
        }
    }
}

impl<'a> pgn_reader::Visitor for PgnPositionVisitor<'a> {
    type Result = ();

    fn begin_game(&mut self) {
        self.pos = Chess::default();
        self.ply = 0;
        let init_hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
        self.accumulator.record(init_hash.0, self.game_id);
    }

    fn begin_variation(&mut self) -> pgn_reader::Skip {
        pgn_reader::Skip(true)
    }

    fn san(&mut self, san_plus: shakmaty::san::SanPlus) {
        if self.ply >= self.max_ply {
            return;
        }

        if let Ok(m) = san_plus.san.to_move(&self.pos) {
            self.pos.play_unchecked(&m);
            self.ply += 1;
            let hash: Zobrist64 = self.pos.zobrist_hash(EnPassantMode::Legal);
            self.accumulator.record(hash.0, self.game_id);
        }
    }

    fn end_game(&mut self) -> Self::Result {}
}

// ---------------------------------------------------------------------------
// Position Index Builders & Disk Serialization
// ---------------------------------------------------------------------------

/// Build static, disk-backed .pos.idx inverted search index for SCID databases in parallel
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)]
pub fn build_for_scid<P1: AsRef<Path>, P2: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
    db_path: P1,
    entries: &[chess_scid_rw::entry::IndexEntry],
    games_path: P2,
    max_ply: usize,
    _max_games: Option<usize>,
    min_games: Option<usize>,
    threads: Option<usize>,
    progress: F,
) -> Result<PositionIndex> {
    let db_p = db_path.as_ref();
    let file = File::open(games_path.as_ref()).with_context(|| {
        format!(
            "Failed to open games file: {}",
            games_path.as_ref().display()
        )
    })?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };

    let total_games = entries.len();
    let chunk_size = 5000;
    let scanned_counter = AtomicUsize::new(0);
    let accumulator = StripedPositionPostingMap::new();

    let run_index = || {
        (0..total_games)
            .into_par_iter()
            .step_by(chunk_size)
            .for_each(|start_idx| {
                let end_idx = (start_idx + chunk_size).min(total_games);

                for game_id in start_idx..end_idx {
                    let entry = &entries[game_id];
                    if entry.deleted {
                        continue;
                    }

                    let start = entry.offset as usize;
                    let end = start + entry.length as usize;
                    if end > mmap.len() || start >= end {
                        continue;
                    }

                    let blob = &mmap[start..end];
                    let mut cursor = 0;

                    let mut pos =
                        match crate::position_search::parse_start_position(blob, &mut cursor) {
                            Some(p) => p,
                            None => continue,
                        };

                    let gid = game_id as u32;
                    let mut ply = 0;

                    let init_hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                    accumulator.record(init_hash.0, gid);

                    let mut slots = crate::position_search::standard_piece_slots();
                    let mut counts = [16usize, 16usize];

                    while cursor < blob.len() && ply < max_ply {
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
                        ply += 1;

                        let hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                        accumulator.record(hash.0, gid);
                    }
                }

                let current_scanned = scanned_counter
                    .fetch_add(end_idx - start_idx, Ordering::Relaxed)
                    + (end_idx - start_idx);
                progress(current_scanned, total_games, accumulator.total_positions());
            });
    };

    if let Some(t) = threads {
        if t > 0 {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(t).build()?;
            pool.install(run_index);
        } else {
            run_index();
        }
    } else {
        run_index();
    }

    let mut positions_map = accumulator.into_map();
    let db_metadata = std::fs::metadata(db_p)?;
    let mtime_secs = db_metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    write_static_binary_file(
        db_p,
        &mut positions_map,
        mtime_secs,
        db_metadata.len(),
        total_games as u64,
        max_ply as u32,
        min_games,
    )?;

    PositionIndex::load(db_p)
}

/// Build static, disk-backed .pos.idx search index for PGN databases in parallel
#[allow(clippy::too_many_arguments)]
#[allow(clippy::needless_range_loop)]
pub fn build_for_pgn<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
    db_path: P,
    entries: &[crate::pgn_db::PgnIndexEntry],
    mmap: &memmap2::Mmap,
    max_ply: usize,
    _max_games: Option<usize>,
    min_games: Option<usize>,
    threads: Option<usize>,
    progress: F,
) -> Result<PositionIndex> {
    let db_p = db_path.as_ref();
    let total_games = entries.len();
    let chunk_size = 5000;
    let scanned_counter = AtomicUsize::new(0);
    let accumulator = StripedPositionPostingMap::new();

    let run_index = || {
        (0..total_games)
            .into_par_iter()
            .step_by(chunk_size)
            .for_each(|start_idx| {
                let end_idx = (start_idx + chunk_size).min(total_games);

                for game_id in start_idx..end_idx {
                    let entry = &entries[game_id];
                    let gid = game_id as u32;

                    let slice = &mmap
                        [entry.offset as usize..(entry.offset as usize + entry.length as usize)];
                    let mut reader = pgn_reader::BufferedReader::new_cursor(slice);
                    let mut visitor = PgnPositionVisitor::new(max_ply, gid, &accumulator);
                    let _ = reader.read_game(&mut visitor);
                }

                let current_scanned = scanned_counter
                    .fetch_add(end_idx - start_idx, Ordering::Relaxed)
                    + (end_idx - start_idx);
                progress(current_scanned, total_games, accumulator.total_positions());
            });
    };

    if let Some(t) = threads {
        if t > 0 {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(t).build()?;
            pool.install(run_index);
        } else {
            run_index();
        }
    } else {
        run_index();
    }

    let mut positions_map = accumulator.into_map();
    let db_metadata = std::fs::metadata(db_p)?;
    let mtime_secs = db_metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    write_static_binary_file(
        db_p,
        &mut positions_map,
        mtime_secs,
        db_metadata.len(),
        total_games as u64,
        max_ply as u32,
        min_games,
    )?;

    PositionIndex::load(db_p)
}

/// Serializes sorted index entries and delta-varint posting lists directly into a static binary file
pub(crate) fn write_static_binary_file(
    db_path: &Path,
    positions_map: &mut HashMap<u64, PositionPostingList>,
    db_mtime_secs: u64,
    db_size_bytes: u64,
    db_game_count: u64,
    max_ply_depth: u32,
    min_games: Option<usize>,
) -> Result<PathBuf> {
    if let Some(min_g) = min_games {
        if min_g > 1 {
            positions_map.retain(|_, posting| posting.games.len() >= min_g);
        }
    }

    let idx_path = PositionIndex::companion_path(db_path);
    let temp_path = idx_path.with_file_name(format!(
        "{}.tmp",
        idx_path.file_name().unwrap_or_default().to_string_lossy()
    ));

    let unique_count = positions_map.len();
    let index_offset = HEADER_SIZE as u64;
    let data_offset =
        index_offset + (unique_count as u64 * std::mem::size_of::<SortedIndexEntry>() as u64);

    let mut hashes: Vec<u64> = positions_map.keys().copied().collect();
    hashes.par_sort_unstable();

    let mut index_entries = Vec::with_capacity(unique_count);
    let mut data_payload = Vec::with_capacity(unique_count * 8);

    for hash in hashes {
        if let Some(posting) = positions_map.remove(&hash) {
            // ⚡ Inlined Singletons Optimization: If position occurs in exactly 1 game,
            // store the Game ID directly in the 32-bit data_offset with INLINE_FLAG!
            // This saves allocating any bytes in the data payload.
            if posting.games.len() == 1 && (posting.games[0] & INLINE_FLAG) == 0 {
                let inlined_offset = INLINE_FLAG | posting.games[0];
                index_entries.push(SortedIndexEntry {
                    hash,
                    data_offset: inlined_offset,
                });
            } else {
                let curr_offset = data_payload.len() as u32;
                let payload_bytes = encode_posting_payload(&posting);
                data_payload.extend_from_slice(&payload_bytes);

                index_entries.push(SortedIndexEntry {
                    hash,
                    data_offset: curr_offset,
                });
            }
        }
    }

    let header = PositionIndexHeader {
        magic: *POS_INDEX_MAGIC,
        version: POS_INDEX_VERSION,
        flags: 0,
        db_mtime_secs,
        db_size_bytes,
        db_game_count,
        max_ply_depth,
        unique_positions: unique_count as u32,
        index_offset,
        data_offset,
        created_timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    {
        let file = File::create(&temp_path).with_context(|| {
            format!(
                "Failed to create temporary position index file: {}",
                temp_path.display()
            )
        })?;
        let mut writer = BufWriter::with_capacity(2 * 1024 * 1024, file);

        header.write_to(&mut writer)?;

        for entry in &index_entries {
            writer.write_all(&entry.hash.to_le_bytes())?;
            writer.write_all(&entry.data_offset.to_le_bytes())?;
        }

        writer.write_all(&data_payload)?;
        writer.flush()?;
    }

    if idx_path.exists() {
        let _ = std::fs::remove_file(&idx_path);
    }
    std::fs::rename(&temp_path, &idx_path).with_context(|| {
        format!(
            "Failed to rename {} to {}",
            temp_path.display(),
            idx_path.display()
        )
    })?;

    eprintln!(
        "[PositionIndex] Successfully saved pure inverted search index: {} ({} unique positions, {:.2} MB)",
        idx_path.display(),
        unique_count,
        std::fs::metadata(&idx_path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0)
    );
    Ok(idx_path)
}
