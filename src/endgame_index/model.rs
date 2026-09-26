use serde::{Deserialize, Serialize};

pub const FEATURE_INDEX_MAGIC: &[u8; 8] = b"CHSFEAT1";
pub const FEATURE_INDEX_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 64;
pub const RECORD_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct GameFeatureRecord {
    pub endgame_bits: u64,
}

impl GameFeatureRecord {
    #[inline]
    pub fn new() -> Self {
        Self { endgame_bits: 0 }
    }

    #[inline]
    pub fn has_endgame_bit(&self, bit: u8) -> bool {
        if bit >= 64 {
            false
        } else {
            (self.endgame_bits & (1u64 << bit)) != 0
        }
    }

    #[inline]
    pub fn set_endgame_bit(&mut self, bit: u8) {
        if bit < 64 {
            self.endgame_bits |= 1u64 << bit;
        }
    }

    #[inline]
    pub fn matches_endgame_mask(&self, mask: u64) -> bool {
        (self.endgame_bits & mask) == mask
    }

    #[inline]
    pub fn matches_any_endgame_mask(&self, mask: u64) -> bool {
        (self.endgame_bits & mask) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub struct FeatureIndexHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub catalog_version: u32,
    pub game_count: u32,
    pub endgame_bit_count: u16,
    pub record_size: u16,
    pub db_mtime_secs: u64,
    pub db_file_size: u64,
    pub created_timestamp: u64,
    pub _reserved: [u8; 16],
}

impl FeatureIndexHeader {
    pub fn new(
        catalog_version: u32,
        game_count: u32,
        endgame_bit_count: u16,
        db_mtime_secs: u64,
        db_file_size: u64,
        created_timestamp: u64,
    ) -> Self {
        let mut magic = [0u8; 8];
        magic.copy_from_slice(FEATURE_INDEX_MAGIC);
        Self {
            magic,
            version: FEATURE_INDEX_VERSION,
            catalog_version,
            game_count,
            endgame_bit_count,
            record_size: RECORD_SIZE as u16,
            db_mtime_secs,
            db_file_size,
            created_timestamp,
            _reserved: [0u8; 16],
        }
    }

    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..8].copy_from_slice(&self.magic);
        buf[8..12].copy_from_slice(&self.version.to_le_bytes());
        buf[12..16].copy_from_slice(&self.catalog_version.to_le_bytes());
        buf[16..20].copy_from_slice(&self.game_count.to_le_bytes());
        buf[20..22].copy_from_slice(&self.endgame_bit_count.to_le_bytes());
        buf[22..24].copy_from_slice(&self.record_size.to_le_bytes());
        buf[24..32].copy_from_slice(&self.db_mtime_secs.to_le_bytes());
        buf[32..40].copy_from_slice(&self.db_file_size.to_le_bytes());
        buf[40..48].copy_from_slice(&self.created_timestamp.to_le_bytes());
        buf[48..64].copy_from_slice(&self._reserved);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_SIZE {
            return None;
        }
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        if &magic != FEATURE_INDEX_MAGIC {
            return None;
        }

        let version = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
        let catalog_version = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
        let game_count = u32::from_le_bytes(bytes[16..20].try_into().ok()?);
        let endgame_bit_count = u16::from_le_bytes(bytes[20..22].try_into().ok()?);
        let record_size = u16::from_le_bytes(bytes[22..24].try_into().ok()?);
        let db_mtime_secs = u64::from_le_bytes(bytes[24..32].try_into().ok()?);
        let db_file_size = u64::from_le_bytes(bytes[32..40].try_into().ok()?);
        let created_timestamp = u64::from_le_bytes(bytes[40..48].try_into().ok()?);
        let mut _reserved = [0u8; 16];
        _reserved.copy_from_slice(&bytes[48..64]);

        Some(Self {
            magic,
            version,
            catalog_version,
            game_count,
            endgame_bit_count,
            record_size,
            db_mtime_secs,
            db_file_size,
            created_timestamp,
            _reserved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_index_header_roundtrip() {
        let header = FeatureIndexHeader::new(2, 50000, 47, 1727312000, 10485760, 1727312005);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);

        let parsed = FeatureIndexHeader::from_bytes(&bytes).expect("Should parse header correctly");
        assert_eq!(parsed, header);
        assert_eq!(parsed.game_count, 50000);
        assert_eq!(parsed.catalog_version, 2);
        assert_eq!(parsed.endgame_bit_count, 47);
        assert_eq!(parsed.record_size, 8);
    }

    #[test]
    fn test_game_feature_record_bits() {
        let mut rec = GameFeatureRecord::new();
        assert_eq!(rec.endgame_bits, 0);

        rec.set_endgame_bit(0);
        rec.set_endgame_bit(5);
        rec.set_endgame_bit(46);

        assert!(rec.has_endgame_bit(0));
        assert!(rec.has_endgame_bit(5));
        assert!(rec.has_endgame_bit(46));
        assert!(!rec.has_endgame_bit(1));
        assert!(!rec.has_endgame_bit(63));

        assert!(rec.matches_endgame_mask(1 | (1 << 5)));
        assert!(!rec.matches_endgame_mask(1 | (1 << 2)));
        assert!(rec.matches_any_endgame_mask(1 << 2 | 1 << 5));
    }
}
