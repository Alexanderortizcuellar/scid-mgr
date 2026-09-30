use anyhow::{bail, Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::{BoostGameEntry, BoostHeader, BoostMove, GAME_ENTRY_SIZE, HEADER_SIZE};

/// Resolves the canonical companion `.boost.idx` path for any SCID or PGN database
pub fn resolve_companion_booster_path<P: AsRef<Path>>(db_path: P) -> PathBuf {
    let p = db_path.as_ref();
    let path_str = p.to_string_lossy();
    let direct = PathBuf::from(format!("{}.boost.idx", path_str));
    if direct.exists() {
        return direct;
    }
    let stem = p.with_extension("boost.idx");
    if stem.exists() {
        return stem;
    }
    stem
}

/// Zero-copy memory-mapped search booster index (`.boost.idx`)
pub struct MmapBoostIndex {
    _mmap: Arc<Mmap>,
    pub header: BoostHeader,
    entries_ptr: *const BoostGameEntry,
    moves_ptr: *const BoostMove,
    game_count: usize,
    total_moves: usize,
}

unsafe impl Send for MmapBoostIndex {}
unsafe impl Sync for MmapBoostIndex {}

impl MmapBoostIndex {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref()).with_context(|| {
            format!("Failed to open search booster index at {:?}", path.as_ref())
        })?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < HEADER_SIZE {
            bail!("Search booster file too small: {} bytes", mmap.len());
        }

        let mut header_buf = [0u8; HEADER_SIZE];
        header_buf.copy_from_slice(&mmap[0..HEADER_SIZE]);

        let header = BoostHeader::from_bytes(&header_buf)
            .ok_or_else(|| anyhow::anyhow!("Invalid search booster header magic or version"))?;

        let dir_offset = header.directory_offset as usize;
        let payload_offset = header.payload_offset as usize;
        let game_count = header.db_game_count as usize;
        let total_moves = header.total_plies as usize;

        let dir_end = dir_offset + game_count * GAME_ENTRY_SIZE;
        let payload_end = payload_offset + total_moves * std::mem::size_of::<BoostMove>();

        if dir_end > mmap.len() || payload_end > mmap.len() {
            bail!(
                "Search booster file length mismatch: file is {} bytes, required at least {}",
                mmap.len(),
                dir_end.max(payload_end)
            );
        }

        let entries_ptr = mmap.as_ptr().wrapping_add(dir_offset) as *const BoostGameEntry;
        let moves_ptr = mmap.as_ptr().wrapping_add(payload_offset) as *const BoostMove;

        Ok(Self {
            _mmap: Arc::new(mmap),
            header,
            entries_ptr,
            moves_ptr,
            game_count,
            total_moves,
        })
    }

    /// Fast header-only verification of companion `.boost.idx` (< 0.001 ms)
    pub fn check_status<P: AsRef<Path>>(
        db_path: P,
        expected_game_count: usize,
    ) -> (crate::position_index::IndexStatus, Option<BoostHeader>) {
        let p = db_path.as_ref();
        let boost_path = resolve_companion_booster_path(p);
        if !boost_path.exists() {
            return (crate::position_index::IndexStatus::Missing, None);
        }

        let mut file = match File::open(&boost_path) {
            Ok(f) => f,
            Err(_) => return (crate::position_index::IndexStatus::Missing, None),
        };

        let mut header_buf = [0u8; HEADER_SIZE];
        if std::io::Read::read_exact(&mut file, &mut header_buf).is_err() {
            return (crate::position_index::IndexStatus::Outdated, None);
        }

        let header = match BoostHeader::from_bytes(&header_buf) {
            Some(h) => h,
            None => return (crate::position_index::IndexStatus::Outdated, None),
        };

        if header.db_game_count as usize != expected_game_count {
            return (crate::position_index::IndexStatus::Outdated, Some(header));
        }

        (crate::position_index::IndexStatus::Valid, Some(header))
    }

    #[inline(always)]
    pub fn header(&self) -> &BoostHeader {
        &self.header
    }

    #[inline(always)]
    pub fn game_count(&self) -> usize {
        self.game_count
    }

    #[inline(always)]
    pub fn num_games(&self) -> usize {
        self.game_count
    }

    #[inline(always)]
    pub fn total_plies(&self) -> u64 {
        self.header.total_plies
    }

    #[inline(always)]
    pub fn total_moves(&self) -> u64 {
        self.header.total_plies
    }

    #[inline(always)]
    pub fn get_game_entry(&self, game_id: usize) -> Option<BoostGameEntry> {
        if game_id >= self.game_count {
            return None;
        }
        unsafe { Some(*self.entries_ptr.add(game_id)) }
    }

    #[inline(always)]
    pub fn game_entry(&self, game_id: usize) -> Option<BoostGameEntry> {
        self.get_game_entry(game_id)
    }

    #[inline(always)]
    pub fn get_game_moves(&self, game_id: usize) -> Option<&[BoostMove]> {
        if game_id >= self.game_count {
            return None;
        }
        let entry = unsafe { *self.entries_ptr.add(game_id) };
        let start = entry.move_offset as usize;
        let count = entry.ply_count as usize;
        if start + count <= self.total_moves {
            unsafe { Some(std::slice::from_raw_parts(self.moves_ptr.add(start), count)) }
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn game_moves(&self, game_id: usize) -> Option<&[BoostMove]> {
        self.get_game_moves(game_id)
    }
}
