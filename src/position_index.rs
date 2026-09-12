use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use shakmaty::fen::Fen;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position};

pub const POS_INDEX_MAGIC: &[u8; 8] = b"SCIDPOS5";
pub const POS_INDEX_VERSION: u32 = 5;
pub const DEFAULT_MAX_SEARCH_PLY: usize = 250;
pub const INLINE_FLAG: u32 = 0x8000_0000;
const NUM_STRIPES: usize = 256;
const HEADER_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus {
    Valid,
    Outdated,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionIndexHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub flags: u32,
    pub db_mtime_secs: u64,
    pub db_size_bytes: u64,
    pub db_game_count: u64,
    pub max_ply_depth: u32,
    pub unique_positions: u32,
    pub index_offset: u64,
    pub data_offset: u64,
    pub created_timestamp: u64,
}

impl PositionIndexHeader {
    pub fn read_from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            anyhow::bail!("Header slice too small");
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if &magic != POS_INDEX_MAGIC {
            anyhow::bail!("Invalid magic bytes in position search index");
        }

        let version = u32::from_le_bytes(bytes[8..12].try_into()?);
        let flags = u32::from_le_bytes(bytes[12..16].try_into()?);
        let db_mtime_secs = u64::from_le_bytes(bytes[16..24].try_into()?);
        let db_size_bytes = u64::from_le_bytes(bytes[24..32].try_into()?);
        let db_game_count = u64::from_le_bytes(bytes[32..40].try_into()?);
        let max_ply_depth = u32::from_le_bytes(bytes[40..44].try_into()?);
        let unique_positions = u32::from_le_bytes(bytes[44..48].try_into()?);
        let index_offset = u64::from_le_bytes(bytes[48..56].try_into()?);
        let data_offset = u64::from_le_bytes(bytes[56..64].try_into()?);

        Ok(Self {
            magic,
            version,
            flags,
            db_mtime_secs,
            db_size_bytes,
            db_game_count,
            max_ply_depth,
            unique_positions,
            index_offset,
            data_offset,
            created_timestamp: 0,
        })
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(&self.magic)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.flags.to_le_bytes())?;
        w.write_all(&self.db_mtime_secs.to_le_bytes())?;
        w.write_all(&self.db_size_bytes.to_le_bytes())?;
        w.write_all(&self.db_game_count.to_le_bytes())?;
        w.write_all(&self.max_ply_depth.to_le_bytes())?;
        w.write_all(&self.unique_positions.to_le_bytes())?;
        w.write_all(&self.index_offset.to_le_bytes())?;
        w.write_all(&self.data_offset.to_le_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C, packed)]
pub struct SortedIndexEntry {
    pub hash: u64,
    pub data_offset: u32,
}

#[derive(Debug, Clone, Default)]
pub struct PositionPostingList {
    pub zobrist_hash: u64,
    pub games: Vec<u32>,
}

impl PositionPostingList {
    pub fn new(zobrist_hash: u64) -> Self {
        Self {
            zobrist_hash,
            games: Vec::new(),
        }
    }

    #[inline]
    pub fn add(&mut self, game_id: u32) {
        if self.games.last().copied() == Some(game_id) {
            return;
        }
        self.games.push(game_id);
    }

    pub fn merge(&mut self, other: PositionPostingList) {
        self.games.extend(other.games);
        self.games.sort_unstable();
        self.games.dedup();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexDiagnostics {
    pub total_positions: usize,
    pub total_postings: usize,
    pub delta_varint_count: usize,
    pub inlined_singletons: usize,
    pub total_game_sets: usize,
    pub bytes_payload: usize,
    pub bucket_1_10: usize,
    pub bucket_11_100: usize,
    pub bucket_101_1k: usize,
    pub bucket_1k_10k: usize,
    pub bucket_10k_100k: usize,
    pub bucket_100k_plus: usize,
}

/// Static, disk-backed position search index using zero-copy memory mapping (`memmap2`)
pub struct PositionIndex {
    pub path: PathBuf,
    mmap: memmap2::Mmap,
    pub header: PositionIndexHeader,
}

impl PositionIndex {
    /// Determines companion `.pos.idx` path for any database file
    pub fn companion_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
        let p = db_path.as_ref();
        let path_str = p.to_string_lossy();
        let lower = path_str.to_lowercase();
        if lower.ends_with(".si5")
            || lower.ends_with(".si4")
            || lower.ends_with(".sg5")
            || lower.ends_with(".sg4")
            || lower.ends_with(".sn5")
            || lower.ends_with(".sn4")
        {
            p.with_extension("pos.idx")
        } else if lower.ends_with(".pgn") {
            PathBuf::from(format!("{}.pos.idx", path_str))
        } else {
            p.with_extension("pos.idx")
        }
    }

    /// Checks if a companion .pos.idx exists and is valid without reading data into RAM (< 0.001 ms)
    pub fn check_status<P: AsRef<Path>>(
        db_path: P,
        expected_game_count: usize,
    ) -> (IndexStatus, Option<PositionIndexHeader>) {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);

        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.pos.idx", p.to_string_lossy()));
            if alt.exists() {
                alt
            } else {
                return (IndexStatus::Missing, None);
            }
        };

        let file = match File::open(&actual_path) {
            Ok(f) => f,
            Err(_) => return (IndexStatus::Missing, None),
        };

        let mut header_buf = [0u8; HEADER_SIZE];
        let mut f_read = file;
        if f_read.read_exact(&mut header_buf).is_err() {
            return (IndexStatus::Outdated, None);
        }

        let header = match PositionIndexHeader::read_from_slice(&header_buf) {
            Ok(h) => h,
            Err(_) => return (IndexStatus::Outdated, None),
        };

        let db_metadata = match std::fs::metadata(p) {
            Ok(m) => m,
            Err(_) => return (IndexStatus::Outdated, Some(header)),
        };

        let current_mtime = db_metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if header.version != POS_INDEX_VERSION
            || header.db_game_count != expected_game_count as u64
            || header.db_mtime_secs != current_mtime
        {
            return (IndexStatus::Outdated, Some(header));
        }

        (IndexStatus::Valid, Some(header))
    }

    /// Open disk-backed position search index via zero-copy mmap (0 MB heap memory allocated)
    pub fn load<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);
        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.pos.idx", p.to_string_lossy()));
            if alt.exists() {
                alt
            } else {
                anyhow::bail!("Position index not found: {}", idx_path.display());
            }
        };

        let file = File::open(&actual_path)
            .with_context(|| format!("Failed to open position index: {}", actual_path.display()))?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let header = PositionIndexHeader::read_from_slice(&mmap[0..HEADER_SIZE])?;
        if header.version != POS_INDEX_VERSION {
            anyhow::bail!(
                "Unsupported position index version: {} (expected {})",
                header.version,
                POS_INDEX_VERSION
            );
        }

        Ok(Self {
            path: actual_path,
            mmap,
            header,
        })
    }

    /// Read slice of sorted index entries directly from memory map
    #[inline]
    pub fn index_entries(&self) -> &[SortedIndexEntry] {
        let start = self.header.index_offset as usize;
        let count = self.header.unique_positions as usize;
        let byte_len = count * std::mem::size_of::<SortedIndexEntry>();
        if start + byte_len > self.mmap.len() {
            return &[];
        }
        let slice = &self.mmap[start..start + byte_len];
        unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const SortedIndexEntry, count) }
    }

    /// Retrieve all matching Game IDs for a given position hash in O(log N) in < 0.01 ms
    pub fn get_matching_game_ids(&self, target_hash: u64) -> Option<Vec<usize>> {
        let entries = self.index_entries();
        let idx = entries
            .binary_search_by_key(&target_hash, |e| e.hash)
            .ok()?;
        let entry = &entries[idx];

        // ⚡ Fast path: Inlined singletons (0 byte payload, instant decode)
        if (entry.data_offset & INLINE_FLAG) != 0 {
            let game_id = (entry.data_offset & !INLINE_FLAG) as usize;
            return Some(vec![game_id]);
        }

        let start = (self.header.data_offset + entry.data_offset as u64) as usize;
        if start >= self.mmap.len() {
            return None;
        }

        let payload = &self.mmap[start..];
        decode_position_game_ids(payload).ok()
    }

    /// Retrieve all matching Game IDs for a given position hash (alias for get_matching_game_ids)
    #[inline]
    pub fn get_all_position_games(&self, target_hash: u64) -> Option<Vec<usize>> {
        self.get_matching_game_ids(target_hash)
    }

    /// Scans diagnostics and distribution metrics of encoded postings
    pub fn scan_diagnostics(&self) -> Result<IndexDiagnostics> {
        let entries = self.index_entries();
        let mut diag = IndexDiagnostics {
            total_positions: entries.len(),
            total_game_sets: entries.len(),
            bytes_payload: if self.mmap.len() > self.header.data_offset as usize {
                self.mmap.len() - self.header.data_offset as usize
            } else {
                0
            },
            ..Default::default()
        };

        for entry in entries {
            if (entry.data_offset & INLINE_FLAG) != 0 {
                diag.total_postings += 1;
                diag.inlined_singletons += 1;
                diag.bucket_1_10 += 1;
            } else {
                let start = (self.header.data_offset + entry.data_offset as u64) as usize;
                if start < self.mmap.len() {
                    let payload = &self.mmap[start..];
                    if let Ok(gids) = decode_position_game_ids(payload) {
                        let count = gids.len();
                        diag.total_postings += count;
                        diag.delta_varint_count += 1;
                        match count {
                            1..=10 => diag.bucket_1_10 += 1,
                            11..=100 => diag.bucket_11_100 += 1,
                            101..=1_000 => diag.bucket_101_1k += 1,
                            1_001..=10_000 => diag.bucket_1k_10k += 1,
                            10_001..=100_000 => diag.bucket_10k_100k += 1,
                            _ => diag.bucket_100k_plus += 1,
                        }
                    }
                }
            }
        }

        Ok(diag)
    }

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
    ) -> Result<Self> {
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

        Self::write_static_binary_file(
            db_p,
            &mut positions_map,
            mtime_secs,
            db_metadata.len(),
            total_games as u64,
            max_ply as u32,
            min_games,
        )?;

        Self::load(db_p)
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
    ) -> Result<Self> {
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

                        let slice = &mmap[entry.offset as usize
                            ..(entry.offset as usize + entry.length as usize)];
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

        Self::write_static_binary_file(
            db_p,
            &mut positions_map,
            mtime_secs,
            db_metadata.len(),
            total_games as u64,
            max_ply as u32,
            min_games,
        )?;

        Self::load(db_p)
    }

    /// Serializes sorted index entries and delta-varint posting lists directly into a static binary file
    fn write_static_binary_file(
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

        let idx_path = Self::companion_path(db_path);
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
}

pub fn parse_target_position(fen_str: &str) -> Option<(Chess, u64)> {
    let trimmed = fen_str.trim();
    if trimmed.is_empty() {
        let pos = Chess::default();
        let hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
        return Some((pos, hash.0));
    }

    if let Ok(fen) = trimmed.parse::<Fen>() {
        if let Ok(pos) = fen.into_position::<Chess>(CastlingMode::Chess960) {
            let hash: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
            return Some((pos, hash.0));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Thread-Safe Striped Map for Parallel Indexing
// ---------------------------------------------------------------------------

struct StripedPositionPostingMap {
    stripes: Vec<Mutex<HashMap<u64, PositionPostingList>>>,
}

impl StripedPositionPostingMap {
    fn new() -> Self {
        let mut stripes = Vec::with_capacity(NUM_STRIPES);
        for _ in 0..NUM_STRIPES {
            stripes.push(Mutex::new(HashMap::with_capacity(1024)));
        }
        Self { stripes }
    }

    #[inline]
    fn stripe_index(hash: u64) -> usize {
        (hash as usize) % NUM_STRIPES
    }

    fn record(&self, hash: u64, game_id: u32) {
        let idx = Self::stripe_index(hash);
        let mut guard = self.stripes[idx].lock().unwrap();
        let posting = guard
            .entry(hash)
            .or_insert_with(|| PositionPostingList::new(hash));
        posting.add(game_id);
    }

    fn total_positions(&self) -> usize {
        self.stripes.iter().map(|s| s.lock().unwrap().len()).sum()
    }

    fn into_map(self) -> HashMap<u64, PositionPostingList> {
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

struct PgnPositionVisitor<'a> {
    max_ply: usize,
    game_id: u32,
    accumulator: &'a StripedPositionPostingMap,
    pos: Chess,
    ply: usize,
}

impl<'a> PgnPositionVisitor<'a> {
    fn new(max_ply: usize, game_id: u32, accumulator: &'a StripedPositionPostingMap) -> Self {
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
// Inverted Index Posting Payload Serialization
// ---------------------------------------------------------------------------

pub fn encode_posting_payload(posting: &PositionPostingList) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + posting.games.len() * 3);
    let count = posting.games.len();
    write_varint(&mut buf, count as u64);

    let mut prev_id = 0u32;
    for &g in &posting.games {
        let delta = g.wrapping_sub(prev_id);
        write_varint(&mut buf, delta as u64);
        prev_id = g;
    }

    buf
}

pub fn decode_position_game_ids(mut slice: &[u8]) -> Result<Vec<usize>> {
    let count = read_varint(&mut slice)? as usize;
    let mut game_ids = Vec::with_capacity(count);

    let mut prev_id = 0u32;
    for _ in 0..count {
        let delta = read_varint(&mut slice)? as u32;
        let id = prev_id.wrapping_add(delta);
        game_ids.push(id as usize);
        prev_id = id;
    }

    Ok(game_ids)
}

// ---------------------------------------------------------------------------
// Compact Varint Helpers
// ---------------------------------------------------------------------------

#[inline]
pub fn write_varint<W: Write>(w: &mut W, mut val: u64) {
    while val >= 0x80 {
        let _ = w.write_all(&[((val & 0x7F) as u8) | 0x80]);
        val >>= 7;
    }
    let _ = w.write_all(&[val as u8]);
}

#[inline]
pub fn read_varint(slice: &mut &[u8]) -> Result<u64> {
    let mut val = 0u64;
    let mut shift = 0;
    while !slice.is_empty() {
        let byte = slice[0];
        *slice = &slice[1..];
        val |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Ok(val);
        }
        shift += 7;
        if shift > 64 {
            anyhow::bail!("Varint overflow");
        }
    }
    anyhow::bail!("Unexpected EOF decoding varint")
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_posting_payload_roundtrip() {
        let mut posting = PositionPostingList::new(123456789);
        posting.add(10);
        posting.add(25);
        posting.add(100);
        posting.add(5000);

        let encoded = encode_posting_payload(&posting);
        let decoded_ids = decode_position_game_ids(&encoded).unwrap();
        assert_eq!(decoded_ids, vec![10, 25, 100, 5000]);
    }
}
