use serde::{Deserialize, Serialize};
use shakmaty::{CastlingSide, Move, Position, Role};

pub const BOOSTER_MAGIC: &[u8; 8] = b"SCIDBST1";
pub const BOOSTER_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 64;
pub const GAME_ENTRY_SIZE: usize = 8;

/// Compact game metadata structure for opening tree and continuation calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BoostGameMeta {
    /// 1 = White Win (1-0), 2 = Black Win (0-1), 3 = Draw (1/2-1/2), other = unknown
    pub result: u8,
    pub white_elo: u16,
    pub black_elo: u16,
    pub year: Option<u16>,
}

impl BoostGameMeta {
    pub fn new(result: u8, white_elo: u16, black_elo: u16, year: Option<u16>) -> Self {
        Self {
            result,
            white_elo,
            black_elo,
            year,
        }
    }
}

/// Compact 16-bit representation of a chess move:
/// - Bits 0..=5   (6 bits): Destination Square (0..=63)
/// - Bits 6..=11  (6 bits): Origin Square (0..=63)
/// - Bits 12..=15 (4 bits): Move Flags (Quiet, Castle, Capture, En Passant, Promotions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct BoostMove(pub u16);

impl BoostMove {
    #[inline(always)]
    pub const fn new(from: u8, to: u8, flags: u8) -> Self {
        Self(((flags as u16 & 0x0F) << 12) | ((from as u16 & 0x3F) << 6) | (to as u16 & 0x3F))
    }

    #[inline(always)]
    pub const fn to(self) -> usize {
        (self.0 & 0x3F) as usize
    }

    #[inline(always)]
    pub const fn from(self) -> usize {
        ((self.0 >> 6) & 0x3F) as usize
    }

    #[inline(always)]
    pub const fn flags(self) -> u8 {
        ((self.0 >> 12) & 0x0F) as u8
    }

    #[inline(always)]
    pub const fn is_quiet(self) -> bool {
        self.flags() == 0x0
    }

    #[inline(always)]
    pub const fn is_double_pawn_push(self) -> bool {
        self.flags() == 0x1
    }

    #[inline(always)]
    pub const fn is_castle_kingside(self) -> bool {
        self.flags() == 0x2
    }

    #[inline(always)]
    pub const fn is_castle_queenside(self) -> bool {
        self.flags() == 0x3
    }

    #[inline(always)]
    pub const fn is_castle(self) -> bool {
        let f = self.flags();
        f == 0x2 || f == 0x3
    }

    #[inline(always)]
    pub const fn is_standard_capture(self) -> bool {
        self.flags() == 0x4
    }

    #[inline(always)]
    pub const fn is_en_passant(self) -> bool {
        self.flags() == 0x5
    }

    #[inline(always)]
    pub const fn is_capture(self) -> bool {
        let f = self.flags();
        f == 0x4 || f == 0x5 || f >= 0xC
    }

    #[inline(always)]
    pub const fn is_promotion(self) -> bool {
        self.flags() >= 0x8
    }

    #[inline(always)]
    pub const fn promotion_role(self) -> Option<Role> {
        match self.flags() & 0x3 {
            0 if self.is_promotion() => Some(Role::Knight),
            1 if self.is_promotion() => Some(Role::Bishop),
            2 if self.is_promotion() => Some(Role::Rook),
            3 if self.is_promotion() => Some(Role::Queen),
            _ => None,
        }
    }

    pub fn to_uci_string(self) -> String {
        let from_sq = shakmaty::Square::new(self.from() as u32);
        let to_sq = shakmaty::Square::new(self.to() as u32);
        let promo_str = match self.promotion_role() {
            Some(Role::Knight) => "n",
            Some(Role::Bishop) => "b",
            Some(Role::Rook) => "r",
            Some(Role::Queen) => "q",
            _ => "",
        };
        format!("{}{}{}", from_sq, to_sq, promo_str)
    }

    /// Converts a standard `shakmaty::Move` into a high-speed 16-bit `BoostMove`
    pub fn from_shakmaty(m: &Move) -> Self {
        let from = m.from().map(|s| s as u8).unwrap_or(0);
        let to = if let Move::Castle { king, rook } = *m {
            if rook.file() > king.file() {
                // King-side
                if king.rank() == shakmaty::Rank::First {
                    6
                } else {
                    62
                }
            } else {
                // Queen-side
                if king.rank() == shakmaty::Rank::First {
                    2
                } else {
                    58
                }
            }
        } else {
            m.to() as u8
        };

        let is_double_pawn_push = if let Move::Normal {
            role: Role::Pawn,
            from: f,
            to: t,
            ..
        } = m
        {
            (f.rank() as i8 - t.rank() as i8).abs() == 2
        } else {
            false
        };

        let flags = if m.is_en_passant() {
            0x5
        } else if m.is_castle() {
            if m.castling_side() == Some(CastlingSide::KingSide) {
                0x2
            } else {
                0x3
            }
        } else if let Some(promo) = m.promotion() {
            let base = match promo {
                Role::Knight => 0x8,
                Role::Bishop => 0x9,
                Role::Rook => 0xA,
                Role::Queen => 0xB,
                _ => 0xB,
            };
            if m.is_capture() {
                base + 4
            } else {
                base
            }
        } else if m.is_capture() {
            0x4
        } else if is_double_pawn_push {
            0x1
        } else {
            0x0
        };

        Self::new(from, to, flags)
    }

    /// Converts a `BoostMove` into a legal `shakmaty::Move` given the current position
    pub fn to_shakmaty_move(self, pos: &shakmaty::Chess) -> Option<shakmaty::Move> {
        let from_sq = shakmaty::Square::new(self.from() as u32);
        let to_sq = shakmaty::Square::new(self.to() as u32);
        let promo = self.promotion_role();

        if self.is_castle_kingside() {
            pos.legal_moves().into_iter().find(|m| {
                m.is_castle() && m.castling_side() == Some(shakmaty::CastlingSide::KingSide)
            })
        } else if self.is_castle_queenside() {
            pos.legal_moves().into_iter().find(|m| {
                m.is_castle() && m.castling_side() == Some(shakmaty::CastlingSide::QueenSide)
            })
        } else {
            pos.legal_moves()
                .into_iter()
                .find(|m| m.from() == Some(from_sq) && m.to() == to_sq && m.promotion() == promo)
        }
    }

    /// Generates standard SAN string (e.g. "e4", "Nf3+", "O-O") given the position
    pub fn to_san_string(self, pos: &shakmaty::Chess) -> Option<String> {
        let mv = self.to_shakmaty_move(pos)?;
        let mut child = pos.clone();
        let san = shakmaty::san::SanPlus::from_move_and_play_unchecked(&mut child, &mv);
        Some(san.to_string())
    }
}

/// 64-byte Header for the `.boost.idx` binary file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub struct BoostHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub header_size: u32,
    pub db_game_count: u32,
    pub flags: u32,
    pub total_plies: u64,
    pub db_mtime_secs: u64,
    pub db_file_size: u64,
    pub directory_offset: u64,
    pub payload_offset: u64,
}

impl BoostHeader {
    pub fn new(
        db_game_count: u32,
        total_plies: u64,
        db_mtime_secs: u64,
        db_file_size: u64,
        directory_offset: u64,
        payload_offset: u64,
    ) -> Self {
        Self {
            magic: *BOOSTER_MAGIC,
            version: BOOSTER_VERSION,
            header_size: HEADER_SIZE as u32,
            db_game_count,
            flags: 0,
            total_plies,
            db_mtime_secs,
            db_file_size,
            directory_offset,
            payload_offset,
        }
    }

    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..8].copy_from_slice(&self.magic);
        buf[8..12].copy_from_slice(&self.version.to_le_bytes());
        buf[12..16].copy_from_slice(&self.header_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.db_game_count.to_le_bytes());
        buf[20..24].copy_from_slice(&self.flags.to_le_bytes());
        buf[24..32].copy_from_slice(&self.total_plies.to_le_bytes());
        buf[32..40].copy_from_slice(&self.db_mtime_secs.to_le_bytes());
        buf[40..48].copy_from_slice(&self.db_file_size.to_le_bytes());
        buf[48..56].copy_from_slice(&self.directory_offset.to_le_bytes());
        buf[56..64].copy_from_slice(&self.payload_offset.to_le_bytes());
        buf
    }

    pub fn from_bytes(slice: &[u8]) -> Option<Self> {
        if slice.len() < HEADER_SIZE || &slice[0..8] != BOOSTER_MAGIC {
            return None;
        }

        let version = u32::from_le_bytes(slice[8..12].try_into().ok()?);
        if version != BOOSTER_VERSION {
            return None;
        }

        let header_size = u32::from_le_bytes(slice[12..16].try_into().ok()?);
        let db_game_count = u32::from_le_bytes(slice[16..20].try_into().ok()?);
        let flags = u32::from_le_bytes(slice[20..24].try_into().ok()?);
        let total_plies = u64::from_le_bytes(slice[24..32].try_into().ok()?);
        let db_mtime_secs = u64::from_le_bytes(slice[32..40].try_into().ok()?);
        let db_file_size = u64::from_le_bytes(slice[40..48].try_into().ok()?);
        let directory_offset = u64::from_le_bytes(slice[48..56].try_into().ok()?);
        let payload_offset = u64::from_le_bytes(slice[56..64].try_into().ok()?);

        Some(Self {
            magic: *BOOSTER_MAGIC,
            version,
            header_size,
            db_game_count,
            flags,
            total_plies,
            db_mtime_secs,
            db_file_size,
            directory_offset,
            payload_offset,
        })
    }
}

/// 8-byte Directory Entry per game enabling O(1) random access and balanced parallel chunking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct BoostGameEntry {
    /// Zero-based index into the move payload array (u16 offset)
    pub move_offset: u32,
    /// Number of half-moves in the game
    pub ply_count: u16,
    /// Result (0: Unknown, 1: 1-0 White, 2: 0-1 Black, 3: 1/2-1/2 Draw)
    pub result: u8,
    /// Status flags (Bit 0: is_deleted, Bits 1..7: reserved)
    pub flags: u8,
}

impl BoostGameEntry {
    #[inline(always)]
    pub fn new(
        move_offset: u32,
        ply_count: u16,
        result: u8,
        is_deleted: bool,
        is_custom_fen: bool,
    ) -> Self {
        let mut flags = 0u8;
        if is_deleted {
            flags |= 1;
        }
        if is_custom_fen {
            flags |= 2;
        }
        Self {
            move_offset,
            ply_count,
            result,
            flags,
        }
    }

    #[inline(always)]
    pub fn is_deleted(&self) -> bool {
        (self.flags & 1) != 0
    }

    #[inline(always)]
    pub fn is_custom_fen(&self) -> bool {
        (self.flags & 2) != 0
    }

    #[inline(always)]
    pub fn to_bytes(&self) -> [u8; GAME_ENTRY_SIZE] {
        let mut buf = [0u8; GAME_ENTRY_SIZE];
        buf[0..4].copy_from_slice(&self.move_offset.to_le_bytes());
        buf[4..6].copy_from_slice(&self.ply_count.to_le_bytes());
        buf[6] = self.result;
        buf[7] = self.flags;
        buf
    }

    #[inline(always)]
    pub fn from_bytes(slice: &[u8]) -> Self {
        let move_offset = u32::from_le_bytes(slice[0..4].try_into().unwrap());
        let ply_count = u16::from_le_bytes(slice[4..6].try_into().unwrap());
        let result = slice[6];
        let flags = slice[7];
        Self {
            move_offset,
            ply_count,
            result,
            flags,
        }
    }
}
