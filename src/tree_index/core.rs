use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

use super::codec::{decode_tree_position_payload, generate_tree_report, parse_target_position};
use super::types::{
    IndexStatus, OpeningTreeReport, SortedTreeIndexEntry, TreeIndexDiagnostics, TreeIndexHeader,
    TreePositionNode, HEADER_SIZE, TREE_INDEX_VERSION,
};

/// Static, disk-backed Opening Tree Stats index using zero-copy memory mapping (`memmap2`)
pub struct TreeIndex {
    pub path: PathBuf,
    mmap: memmap2::Mmap,
    pub header: TreeIndexHeader,
}

impl TreeIndex {
    /// Determines companion `.tree.idx` path for any database file
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
            p.with_extension("tree.idx")
        } else if lower.ends_with(".pgn") {
            PathBuf::from(format!("{}.tree.idx", path_str))
        } else {
            p.with_extension("tree.idx")
        }
    }

    /// Checks if a companion .tree.idx exists and is valid without reading data into RAM (< 0.001 ms)
    pub fn check_status<P: AsRef<Path>>(
        db_path: P,
        expected_game_count: usize,
    ) -> (IndexStatus, Option<TreeIndexHeader>) {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);

        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.tree.idx", p.to_string_lossy()));
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

        let header = match TreeIndexHeader::read_from_slice(&header_buf) {
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

        if header.version != TREE_INDEX_VERSION
            || header.db_game_count != expected_game_count as u64
            || header.db_mtime_secs != current_mtime
        {
            return (IndexStatus::Outdated, Some(header));
        }

        (IndexStatus::Valid, Some(header))
    }

    /// Open disk-backed tree index via zero-copy mmap (0 MB heap memory allocated)
    pub fn load<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let p = db_path.as_ref();
        let idx_path = Self::companion_path(p);
        let actual_path = if idx_path.exists() {
            idx_path
        } else {
            let alt = PathBuf::from(format!("{}.tree.idx", p.to_string_lossy()));
            if alt.exists() {
                alt
            } else {
                anyhow::bail!("Tree index not found: {}", idx_path.display());
            }
        };

        let file = File::open(&actual_path)
            .with_context(|| format!("Failed to open tree index: {}", actual_path.display()))?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let header = TreeIndexHeader::read_from_slice(&mmap[0..HEADER_SIZE])?;
        if header.version != TREE_INDEX_VERSION {
            anyhow::bail!(
                "Unsupported tree index version: {} (expected {})",
                header.version,
                TREE_INDEX_VERSION
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
    pub fn index_entries(&self) -> &[SortedTreeIndexEntry] {
        let start = self.header.index_offset as usize;
        let count = self.header.unique_positions as usize;
        let byte_len = count * std::mem::size_of::<SortedTreeIndexEntry>();
        if start + byte_len > self.mmap.len() {
            return &[];
        }
        let slice = &self.mmap[start..start + byte_len];
        unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const SortedTreeIndexEntry, count) }
    }

    /// Look up a position node by its 64-bit Zobrist hash in O(log N) directly from mmap (< 0.001 ms)
    pub fn get_position(&self, target_hash: u64) -> Option<TreePositionNode> {
        let entries = self.index_entries();
        let idx = entries
            .binary_search_by_key(&target_hash, |e| e.hash)
            .ok()?;
        let entry = &entries[idx];

        let start = (self.header.data_offset + entry.data_offset as u64) as usize;
        let end = if idx + 1 < entries.len() {
            (self.header.data_offset + entries[idx + 1].data_offset as u64) as usize
        } else {
            self.mmap.len()
        };
        if end > self.mmap.len() || start >= end {
            return None;
        }

        let payload = &self.mmap[start..end];
        decode_tree_position_payload(payload, target_hash).ok()
    }

    /// Query the opening tree for any board position (FEN or standard starting board) with zero heap RAM overhead (< 0.01 ms)
    pub fn query_tree(&self, fen_str: &str) -> Option<OpeningTreeReport> {
        let (pos, zobrist_hash) = parse_target_position(fen_str)?;
        let node = self.get_position(zobrist_hash)?;
        Some(generate_tree_report(&node, &pos, zobrist_hash))
    }

    /// Query the opening tree with optional parameters for API compatibility
    pub fn query_tree_with_options(
        &self,
        fen_str: &str,
        _target_game_ids: Option<&[usize]>,
        _sample_limit: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        self.query_tree(fen_str)
    }

    /// Scans diagnostics and distribution metrics of opening tree index
    pub fn scan_diagnostics(&self) -> Result<TreeIndexDiagnostics> {
        let entries = self.index_entries();
        let mut diag = TreeIndexDiagnostics {
            total_positions: entries.len(),
            ..Default::default()
        };

        for i in 0..entries.len() {
            let start = (self.header.data_offset + entries[i].data_offset as u64) as usize;
            let end = if i + 1 < entries.len() {
                (self.header.data_offset + entries[i + 1].data_offset as u64) as usize
            } else {
                self.mmap.len()
            };
            if start < end && end <= self.mmap.len() {
                let payload = &self.mmap[start..end];
                diag.bytes_total += payload.len();
                if let Ok(node) = decode_tree_position_payload(payload, entries[i].hash) {
                    let count = node.total_games as usize;
                    diag.total_tree_moves += node.moves.len();
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

        Ok(diag)
    }

    /// On-the-fly dynamic calculation of opening tree statistics for SCID databases
    pub fn calculate_tree_for_scid<P: AsRef<Path>>(
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: P,
        fen_str: &str,
        target_game_ids: Option<&[usize]>,
        max_depth: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        super::dynamic::calculate_tree_for_scid(
            entries,
            games_path,
            fen_str,
            target_game_ids,
            max_depth,
        )
    }

    /// On-the-fly dynamic calculation of opening tree statistics for PGN databases
    pub fn calculate_tree_for_pgn(
        entries: &[crate::pgn_db::PgnIndexEntry],
        mmap: &memmap2::Mmap,
        fen_str: &str,
        target_game_ids: Option<&[usize]>,
        max_depth: Option<usize>,
    ) -> Option<OpeningTreeReport> {
        super::dynamic::calculate_tree_for_pgn(entries, mmap, fen_str, target_game_ids, max_depth)
    }

    /// Build static, disk-backed .tree.idx file for SCID databases in parallel across CPU cores
    #[allow(clippy::too_many_arguments)]
    pub fn build_for_scid<P: AsRef<Path>, F: Fn(usize, usize, usize) + Sync>(
        db_path: P,
        entries: &[chess_scid_rw::entry::IndexEntry],
        games_path: P,
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

    /// Build static, disk-backed .tree.idx file for PGN databases in parallel
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
