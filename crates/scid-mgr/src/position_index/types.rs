use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;

pub const POS_INDEX_MAGIC: &[u8; 8] = b"SCIDPOS5";
pub const POS_INDEX_VERSION: u32 = 5;
pub const DEFAULT_MAX_SEARCH_PLY: usize = 250;
pub const INLINE_FLAG: u32 = 0x8000_0000;
pub const NUM_STRIPES: usize = 256;
pub const HEADER_SIZE: usize = 64;

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
