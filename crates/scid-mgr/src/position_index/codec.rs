use anyhow::Result;
use shakmaty::fen::Fen;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, EnPassantMode};
use std::io::Write;

use super::types::PositionPostingList;

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
