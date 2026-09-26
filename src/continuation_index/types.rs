use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess};

pub use crate::tree_index::types::PackedMove;

pub const HOT_GRAPH_MAGIC: &[u8; 8] = b"CHSHOTG1";
pub const HOT_GRAPH_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 96;
pub const NO_NODE: NodeId = u32::MAX;
pub const NUM_BUILDER_STRIPES: usize = 256;

pub type NodeId = u32;

/// Compact contiguous 24-byte node in the hot position graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct HotNode {
    pub first_edge: u32,
    pub edge_count: u16,
    pub _padding: u16,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
}

impl HotNode {
    #[inline]
    pub fn new(
        first_edge: u32,
        edge_count: u16,
        total_games: u32,
        white_wins: u32,
        draws: u32,
        black_wins: u32,
    ) -> Self {
        Self {
            first_edge,
            edge_count,
            _padding: 0,
            total_games,
            white_wins,
            draws,
            black_wins,
        }
    }
}

/// Compact contiguous 24-byte edge representing a move played from a position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct HotEdge {
    pub packed_move: u16,
    pub _padding: u16,
    pub target_node: u32, // NodeId or NO_NODE if beyond hot cutoff
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
}

impl HotEdge {
    #[inline]
    pub fn new(
        packed_move: PackedMove,
        target_node: NodeId,
        total_games: u32,
        white_wins: u32,
        draws: u32,
        black_wins: u32,
    ) -> Self {
        Self {
            packed_move: packed_move.0,
            _padding: 0,
            target_node,
            total_games,
            white_wins,
            draws,
            black_wins,
        }
    }

    #[inline]
    pub fn packed_move(&self) -> PackedMove {
        PackedMove(self.packed_move)
    }
}

/// 12-byte sorted index entry mapping Zobrist hash to NodeId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C, packed)]
pub struct HotHashEntry {
    pub hash: u64,
    pub node_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HotGraphMetadata {
    pub max_ply: u32,
    pub min_games: u32,
    pub db_game_count: u64,
    pub db_mtime_secs: u64,
    pub db_file_size: u64,
    pub created_timestamp: u64,
    pub node_count: u32,
    pub edge_count: u32,
    pub hash_count: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HotGraphHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub flags: u32,
    pub db_mtime_secs: u64,
    pub db_file_size: u64,
    pub db_game_count: u64,
    pub max_ply: u32,
    pub min_games: u32,
    pub node_count: u32,
    pub edge_count: u32,
    pub hash_count: u32,
    pub _reserved: u32,
    pub nodes_offset: u64,
    pub edges_offset: u64,
    pub hashes_offset: u64,
    pub created_timestamp: u64,
}

impl HotGraphHeader {
    pub fn read_from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            bail!(
                "Header slice too small (got {}, expected {})",
                bytes.len(),
                HEADER_SIZE
            );
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if &magic != HOT_GRAPH_MAGIC {
            bail!("Invalid hot graph magic bytes: {:?}", magic);
        }

        let version = u32::from_le_bytes(bytes[8..12].try_into()?);
        let flags = u32::from_le_bytes(bytes[12..16].try_into()?);
        let db_mtime_secs = u64::from_le_bytes(bytes[16..24].try_into()?);
        let db_file_size = u64::from_le_bytes(bytes[24..32].try_into()?);
        let db_game_count = u64::from_le_bytes(bytes[32..40].try_into()?);
        let max_ply = u32::from_le_bytes(bytes[40..44].try_into()?);
        let min_games = u32::from_le_bytes(bytes[44..48].try_into()?);
        let node_count = u32::from_le_bytes(bytes[48..52].try_into()?);
        let edge_count = u32::from_le_bytes(bytes[52..56].try_into()?);
        let hash_count = u32::from_le_bytes(bytes[56..60].try_into()?);
        let _reserved = u32::from_le_bytes(bytes[60..64].try_into()?);
        let nodes_offset = u64::from_le_bytes(bytes[64..72].try_into()?);
        let edges_offset = u64::from_le_bytes(bytes[72..80].try_into()?);
        let hashes_offset = u64::from_le_bytes(bytes[80..88].try_into()?);
        let created_timestamp = u64::from_le_bytes(bytes[88..96].try_into()?);

        Ok(Self {
            magic,
            version,
            flags,
            db_mtime_secs,
            db_file_size,
            db_game_count,
            max_ply,
            min_games,
            node_count,
            edge_count,
            hash_count,
            _reserved,
            nodes_offset,
            edges_offset,
            hashes_offset,
            created_timestamp,
        })
    }

    pub fn write_to<W: std::io::Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(&self.magic)?;
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.flags.to_le_bytes())?;
        w.write_all(&self.db_mtime_secs.to_le_bytes())?;
        w.write_all(&self.db_file_size.to_le_bytes())?;
        w.write_all(&self.db_game_count.to_le_bytes())?;
        w.write_all(&self.max_ply.to_le_bytes())?;
        w.write_all(&self.min_games.to_le_bytes())?;
        w.write_all(&self.node_count.to_le_bytes())?;
        w.write_all(&self.edge_count.to_le_bytes())?;
        w.write_all(&self.hash_count.to_le_bytes())?;
        w.write_all(&self._reserved.to_le_bytes())?;
        w.write_all(&self.nodes_offset.to_le_bytes())?;
        w.write_all(&self.edges_offset.to_le_bytes())?;
        w.write_all(&self.hashes_offset.to_le_bytes())?;
        w.write_all(&self.created_timestamp.to_le_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuationLine {
    pub moves: Vec<String>,
    pub formatted: String,
    pub games: u64,
    pub percentage: f64,
    #[serde(default)]
    pub white_wins: u64,
    #[serde(default)]
    pub draws: u64,
    #[serde(default)]
    pub black_wins: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuationNode {
    pub san: String,
    pub games: u64,
    pub percentage: f64,
    #[serde(default)]
    pub white_wins: u64,
    #[serde(default)]
    pub draws: u64,
    #[serde(default)]
    pub black_wins: u64,
    pub children: Vec<ContinuationNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuationResult {
    pub starting_fen: String,
    pub total_games_processed: u64,
    pub games_reaching_position: u64,
    pub lines: Vec<ContinuationLine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<ContinuationNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationQuery {
    pub position: String,
    pub max_depth: usize,
    pub max_lines: usize,
    pub min_games: u64,
    pub min_percentage: f64,
    pub hot_idx: Option<String>,
    pub pos_idx: Option<String>,
}

impl Default for ContinuationQuery {
    fn default() -> Self {
        Self {
            position: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
            max_depth: 8,
            max_lines: 10,
            min_games: 1,
            min_percentage: 0.0,
            hot_idx: None,
            pos_idx: None,
        }
    }
}

impl ContinuationQuery {
    pub fn validate(&self) -> Result<Chess> {
        if self.max_depth < 1 || self.max_depth > 20 {
            bail!("max_depth must be in range 1..=20, got {}", self.max_depth);
        }

        if self.max_lines == 0 {
            bail!("max_lines must be greater than 0");
        }

        if self.min_percentage < 0.0 || self.min_percentage > 100.0 {
            bail!(
                "min_percentage must be between 0.0 and 100.0, got {}",
                self.min_percentage
            );
        }

        let fen: Fen = self
            .position
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse FEN '{}': {}", self.position, e))?;

        let pos: Chess = fen.into_position(CastlingMode::Standard).map_err(|e| {
            anyhow::anyhow!("Invalid chess position from FEN '{}': {}", self.position, e)
        })?;

        Ok(pos)
    }
}
