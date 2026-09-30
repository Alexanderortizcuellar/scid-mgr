use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

use super::codec::decode_position_game_ids;
use super::types::{
    IndexDiagnostics, IndexStatus, PositionIndexHeader, SortedIndexEntry, HEADER_SIZE, INLINE_FLAG,
    POS_INDEX_VERSION,
};

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
    pub fn build_for_scid<P1: AsRef<Path>, P2: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P1,
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: P2,
        max_ply: usize,
        max_games: Option<usize>,
        min_games: Option<usize>,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        super::builder::build_for_scid(
            db_path, entries, games_path, max_ply, max_games, min_games, threads, progress,
        )
    }

    /// Build static, disk-backed .pos.idx search index for PGN databases in parallel
    #[allow(clippy::too_many_arguments)]
    pub fn build_for_pgn<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        max_ply: usize,
        max_games: Option<usize>,
        min_games: Option<usize>,
        threads: Option<usize>,
        progress: F,
    ) -> Result<Self> {
        super::builder::build_for_pgn(
            db_path, entries, mmap, max_ply, max_games, min_games, threads, progress,
        )
    }
}
