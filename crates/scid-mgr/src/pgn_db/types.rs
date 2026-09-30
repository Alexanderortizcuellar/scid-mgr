use serde::{Deserialize, Serialize};

pub const PGN_INDEX_MAGIC: &[u8; 8] = b"SCIDPGN2";
pub const PGN_INDEX_VERSION: u32 = 1;

/// Packed 40-byte binary record for a single game in a raw .pgn file
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CompactPgnRecord {
    pub offset: u64,    // 8 bytes: byte offset in .pgn
    pub length: u32,    // 4 bytes: byte length of game text
    pub white_id: u32,  // 4 bytes: player name dictionary ID
    pub black_id: u32,  // 4 bytes: player name dictionary ID
    pub event_id: u32,  // 4 bytes: event name dictionary ID
    pub site_id: u32,   // 4 bytes: site name dictionary ID
    pub date: u32,      // 4 bytes: packed (YYYY << 9) | (MM << 5) | DD
    pub eco: u16,       // 2 bytes: packed ECO (0..499, or 0xFFFF)
    pub white_elo: u16, // 2 bytes: Elo (0 = none)
    pub black_elo: u16, // 2 bytes: Elo (0 = none)
    pub result: u8,     // 1 byte: 0=*, 1=1-0, 2=0-1, 3=1/2-1/2
    pub _padding: u8,   // 1 byte: alignment padding (total 40 bytes)
}

pub type PgnIndexEntry = CompactPgnRecord;

impl CompactPgnRecord {
    #[inline]
    pub fn result_str(&self) -> &'static str {
        unpack_result(self.result)
    }

    #[inline]
    pub fn date_str(&self) -> String {
        unpack_date(self.date)
    }

    #[inline]
    pub fn eco_str(&self) -> String {
        unpack_eco(self.eco)
    }
}

/// Deduplicated string dictionary for PGN metadata (Player names, Events, Sites)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PgnNameTables {
    pub players: Vec<String>,
    pub events: Vec<String>,
    pub sites: Vec<String>,
}

impl PgnNameTables {
    pub fn new() -> Self {
        Self {
            players: vec!["?".to_string()],
            events: vec!["?".to_string()],
            sites: vec!["?".to_string()],
        }
    }

    #[inline]
    pub fn player(&self, id: u32) -> &str {
        self.players
            .get(id as usize)
            .map(|s| s.as_str())
            .unwrap_or("?")
    }

    #[inline]
    pub fn event(&self, id: u32) -> &str {
        self.events
            .get(id as usize)
            .map(|s| s.as_str())
            .unwrap_or("?")
    }

    #[inline]
    pub fn site(&self, id: u32) -> &str {
        self.sites
            .get(id as usize)
            .map(|s| s.as_str())
            .unwrap_or("?")
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct PgnIndexHeader {
    pub(crate) magic: [u8; 8],
    pub(crate) version: u32,
    pub(crate) flags: u32,
    pub(crate) pgn_mtime_secs: u64,
    pub(crate) pgn_file_size: u64,
    pub(crate) game_count: u64,
    pub(crate) namebase_offset: u64,
    pub(crate) namebase_len: u64,
    pub(crate) records_offset: u64,
}

pub(crate) struct RawGameRecord<'a> {
    pub(crate) offset: u64,
    pub(crate) length: u32,
    pub(crate) white: &'a str,
    pub(crate) black: &'a str,
    pub(crate) event: &'a str,
    pub(crate) site: &'a str,
    pub(crate) date: u32,
    pub(crate) eco: u16,
    pub(crate) white_elo: u16,
    pub(crate) black_elo: u16,
    pub(crate) result: u8,
}

pub fn pack_date(s: &str) -> u32 {
    let mut parts = s.split('.');
    let year = parts
        .next()
        .and_then(|y| y.parse::<u16>().ok())
        .unwrap_or(0);
    let month = parts.next().and_then(|m| m.parse::<u8>().ok()).unwrap_or(0);
    let day = parts.next().and_then(|d| d.parse::<u8>().ok()).unwrap_or(0);
    ((year as u32) << 9) | (((month & 0x0F) as u32) << 5) | ((day & 0x1F) as u32)
}

pub fn unpack_date(d: u32) -> String {
    let year = (d >> 9) as u16;
    let month = ((d >> 5) & 0x0F) as u8;
    let day = (d & 0x1F) as u8;
    let y_str = if year == 0 {
        "????".to_string()
    } else {
        format!("{:04}", year)
    };
    let m_str = if month == 0 {
        "??".to_string()
    } else {
        format!("{:02}", month)
    };
    let d_str = if day == 0 {
        "??".to_string()
    } else {
        format!("{:02}", day)
    };
    format!("{}.{}.{}", y_str, m_str, d_str)
}

pub fn pack_eco(s: &str) -> u16 {
    let trimmed = s.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3 {
        let l = bytes[0].to_ascii_uppercase();
        if (b'A'..=b'E').contains(&l) {
            let letter_val = (l - b'A') as u16;
            if let Ok(digits) = std::str::from_utf8(&bytes[1..3])
                .unwrap_or("")
                .parse::<u16>()
            {
                if digits < 100 {
                    return letter_val * 100 + digits;
                }
            }
        }
    }
    0xFFFF
}

pub fn unpack_eco(eco: u16) -> String {
    if eco <= 499 {
        let letter = (b'A' + (eco / 100) as u8) as char;
        let digits = eco % 100;
        format!("{}{:02}", letter, digits)
    } else {
        String::new()
    }
}

pub fn pack_result(s: &str) -> u8 {
    match s.trim() {
        "1-0" => 1,
        "0-1" => 2,
        "1/2-1/2" => 3,
        _ => 0,
    }
}

pub fn unpack_result(r: u8) -> &'static str {
    match r {
        1 => "1-0",
        2 => "0-1",
        3 => "1/2-1/2",
        _ => "*",
    }
}
