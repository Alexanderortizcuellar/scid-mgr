use anyhow::Result;
use serde::{Deserialize, Serialize};
use shakmaty::{Chess, Position};
use std::io::Write;

pub const TREE_INDEX_MAGIC: &[u8; 8] = b"SCIDTRE1";
pub const TREE_INDEX_VERSION: u32 = 1;
pub const DEFAULT_MAX_TREE_PLY: usize = 24; // 12 full moves
pub const NUM_STRIPES: usize = 256;
pub const HEADER_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexStatus {
    Valid,
    Outdated,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeIndexDiagnostics {
    pub total_positions: usize,
    pub total_tree_moves: usize,
    pub bytes_total: usize,
    pub bucket_1_10: usize,
    pub bucket_11_100: usize,
    pub bucket_101_1k: usize,
    pub bucket_1k_10k: usize,
    pub bucket_10k_100k: usize,
    pub bucket_100k_plus: usize,
}

/// Compact 16-bit binary move representation:
/// - Bits 0..5: From Square (0..63)
/// - Bits 6..11: To Square (0..63)
/// - Bits 12..14: Promotion Piece (0=None, 1=Knight, 2=Bishop, 3=Rook, 4=Queen)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PackedMove(pub u16);

impl PackedMove {
    #[inline]
    pub fn new(from: u8, to: u8, promo: Option<shakmaty::Role>) -> Self {
        let p = match promo {
            None => 0u16,
            Some(shakmaty::Role::Knight) => 1,
            Some(shakmaty::Role::Bishop) => 2,
            Some(shakmaty::Role::Rook) => 3,
            Some(shakmaty::Role::Queen) => 4,
            _ => 0,
        };
        let val = (from as u16 & 0x3F) | ((to as u16 & 0x3F) << 6) | (p << 12);
        PackedMove(val)
    }

    #[inline]
    pub fn from_square(self) -> u8 {
        (self.0 & 0x3F) as u8
    }

    #[inline]
    pub fn to_square(self) -> u8 {
        ((self.0 >> 6) & 0x3F) as u8
    }

    #[inline]
    pub fn promotion(self) -> Option<shakmaty::Role> {
        match (self.0 >> 12) & 0x07 {
            1 => Some(shakmaty::Role::Knight),
            2 => Some(shakmaty::Role::Bishop),
            3 => Some(shakmaty::Role::Rook),
            4 => Some(shakmaty::Role::Queen),
            _ => None,
        }
    }

    pub fn to_uci_string(self) -> String {
        let from_sq = shakmaty::Square::new(self.from_square() as u32);
        let to_sq = shakmaty::Square::new(self.to_square() as u32);
        let promo_str = match self.promotion() {
            Some(shakmaty::Role::Knight) => "n",
            Some(shakmaty::Role::Bishop) => "b",
            Some(shakmaty::Role::Rook) => "r",
            Some(shakmaty::Role::Queen) => "q",
            _ => "",
        };
        format!("{}{}{}", from_sq, to_sq, promo_str)
    }

    pub fn to_shakmaty_move(self, pos: &Chess) -> Option<shakmaty::Move> {
        let from_sq = shakmaty::Square::new(self.from_square() as u32);
        let to_sq = shakmaty::Square::new(self.to_square() as u32);
        let promo = self.promotion();

        pos.legal_moves()
            .into_iter()
            .find(|m| m.from() == Some(from_sq) && m.to() == to_sq && m.promotion() == promo)
    }
}

impl From<&shakmaty::Move> for PackedMove {
    #[inline]
    fn from(m: &shakmaty::Move) -> Self {
        let from = m.from().map(|sq| sq as u8).unwrap_or(0);
        let to = m.to() as u8;
        PackedMove::new(from, to, m.promotion())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeIndexHeader {
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

impl TreeIndexHeader {
    pub fn read_from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            anyhow::bail!("Header slice too small");
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if &magic != TREE_INDEX_MAGIC {
            anyhow::bail!("Invalid magic bytes in tree index");
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
pub struct SortedTreeIndexEntry {
    pub hash: u64,
    pub data_offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeMoveStats {
    pub packed_move: u16,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub white_elo_sum: u64,
    pub black_elo_sum: u64,
    pub elo_game_count: u32,
}

impl TreeMoveStats {
    pub fn avg_white_elo(&self) -> Option<u32> {
        if self.elo_game_count > 0 {
            Some((self.white_elo_sum / self.elo_game_count as u64) as u32)
        } else {
            None
        }
    }

    pub fn avg_black_elo(&self) -> Option<u32> {
        if self.elo_game_count > 0 {
            Some((self.black_elo_sum / self.elo_game_count as u64) as u32)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreePositionNode {
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub moves: Vec<TreeMoveStats>,
}

impl TreePositionNode {
    pub fn new(zobrist_hash: u64) -> Self {
        Self {
            zobrist_hash,
            total_games: 0,
            white_wins: 0,
            draws: 0,
            black_wins: 0,
            moves: Vec::new(),
        }
    }

    pub fn record_game(
        &mut self,
        next_move: Option<u16>,
        w_win: u32,
        draw: u32,
        b_win: u32,
        w_elo: u16,
        b_elo: u16,
    ) {
        self.total_games += 1;
        self.white_wins += w_win;
        self.draws += draw;
        self.black_wins += b_win;

        if let Some(packed) = next_move {
            let move_stat =
                if let Some(pos) = self.moves.iter().position(|m| m.packed_move == packed) {
                    &mut self.moves[pos]
                } else {
                    self.moves.push(TreeMoveStats {
                        packed_move: packed,
                        total_games: 0,
                        white_wins: 0,
                        draws: 0,
                        black_wins: 0,
                        white_elo_sum: 0,
                        black_elo_sum: 0,
                        elo_game_count: 0,
                    });
                    self.moves.last_mut().unwrap()
                };

            move_stat.total_games += 1;
            move_stat.white_wins += w_win;
            move_stat.draws += draw;
            move_stat.black_wins += b_win;
            if w_elo > 0 && b_elo > 0 {
                move_stat.white_elo_sum += w_elo as u64;
                move_stat.black_elo_sum += b_elo as u64;
                move_stat.elo_game_count += 1;
            }
        }
    }

    pub fn merge(&mut self, other: TreePositionNode) {
        self.total_games += other.total_games;
        self.white_wins += other.white_wins;
        self.draws += other.draws;
        self.black_wins += other.black_wins;

        for other_m in other.moves {
            if let Some(m) = self
                .moves
                .iter_mut()
                .find(|m| m.packed_move == other_m.packed_move)
            {
                m.total_games += other_m.total_games;
                m.white_wins += other_m.white_wins;
                m.draws += other_m.draws;
                m.black_wins += other_m.black_wins;
                m.white_elo_sum += other_m.white_elo_sum;
                m.black_elo_sum += other_m.black_elo_sum;
                m.elo_game_count += other_m.elo_game_count;
            } else {
                self.moves.push(other_m);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTreeMoveView {
    pub san: String,
    pub uci: String,
    pub total_games: u32,
    pub white_pct: f64,
    pub draw_pct: f64,
    pub black_pct: f64,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub avg_white_elo: Option<u32>,
    pub avg_black_elo: Option<u32>,
    #[serde(default)]
    pub last_played: Option<String>,
    pub sample_game_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTreeReport {
    pub fen: String,
    pub zobrist_hash: u64,
    pub total_games: u32,
    pub white_wins: u32,
    pub draws: u32,
    pub black_wins: u32,
    pub white_pct: f64,
    pub draw_pct: f64,
    pub black_pct: f64,
    pub moves: Vec<OpeningTreeMoveView>,
    pub sample_game_ids: Vec<u32>,
    #[serde(default)]
    pub sample_games: Vec<crate::db::GameSummary>,
}
