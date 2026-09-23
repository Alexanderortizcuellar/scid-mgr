use anyhow::Result;
use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, EnPassantMode};
use std::io::Write;

use super::types::{
    OpeningTreeMoveView, OpeningTreeReport, PackedMove, TreeMoveStats, TreePositionNode,
};

// ---------------------------------------------------------------------------
// Compact Binary Payload Encoding & Decoding
// ---------------------------------------------------------------------------

pub fn encode_tree_position_payload(node: &TreePositionNode) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16 + node.moves.len() * 12);
    write_varint(&mut buf, node.total_games as u64);
    write_varint(&mut buf, node.white_wins as u64);
    write_varint(&mut buf, node.black_wins as u64);
    write_varint(&mut buf, node.moves.len() as u64);

    for m in &node.moves {
        buf.extend_from_slice(&m.packed_move.to_le_bytes());
        write_varint(&mut buf, m.white_wins as u64);
        write_varint(&mut buf, m.draws as u64);
        write_varint(&mut buf, m.black_wins as u64);
        let avg_w = m.avg_white_elo().unwrap_or(0) as u16;
        let avg_b = m.avg_black_elo().unwrap_or(0) as u16;
        buf.extend_from_slice(&avg_w.to_le_bytes());
        buf.extend_from_slice(&avg_b.to_le_bytes());
    }

    buf
}

pub fn decode_tree_position_payload(
    mut slice: &[u8],
    zobrist_hash: u64,
) -> Result<TreePositionNode> {
    let total_games = read_varint(&mut slice)? as u32;
    let white_wins = read_varint(&mut slice)? as u32;
    let black_wins = read_varint(&mut slice)? as u32;
    let draws = total_games.saturating_sub(white_wins + black_wins);
    let move_count = read_varint(&mut slice)? as usize;

    let mut moves = Vec::with_capacity(move_count);
    for _ in 0..move_count {
        if slice.len() < 2 {
            anyhow::bail!("Unexpected EOF reading packed_move");
        }
        let packed_move = u16::from_le_bytes(slice[0..2].try_into()?);
        slice = &slice[2..];

        let m_ww = read_varint(&mut slice)? as u32;
        let m_dr = read_varint(&mut slice)? as u32;
        let m_bw = read_varint(&mut slice)? as u32;
        let m_tot = m_ww + m_dr + m_bw;

        if slice.len() < 4 {
            anyhow::bail!("Unexpected EOF reading move elo averages");
        }
        let avg_w = u16::from_le_bytes(slice[0..2].try_into()?);
        let avg_b = u16::from_le_bytes(slice[2..4].try_into()?);
        slice = &slice[4..];

        moves.push(TreeMoveStats {
            packed_move,
            total_games: m_tot,
            white_wins: m_ww,
            draws: m_dr,
            black_wins: m_bw,
            white_elo_sum: avg_w as u64 * m_tot as u64,
            black_elo_sum: avg_b as u64 * m_tot as u64,
            elo_game_count: if avg_w > 0 { m_tot } else { 0 },
        });
    }

    Ok(TreePositionNode {
        zobrist_hash,
        total_games,
        white_wins,
        draws,
        black_wins,
        moves,
    })
}

pub fn generate_tree_report(
    node: &TreePositionNode,
    pos: &Chess,
    zobrist_hash: u64,
) -> OpeningTreeReport {
    let total = node.total_games.max(1);
    let round_2dp = |v: f64| (v * 100.0).round() / 100.0;

    let mut move_views: Vec<OpeningTreeMoveView> = node
        .moves
        .iter()
        .map(|m| {
            let m_total = m.total_games.max(1);
            let packed = PackedMove(m.packed_move);
            let uci = packed.to_uci_string();
            let san = if let Some(shak_move) = packed.to_shakmaty_move(pos) {
                let mut p_copy = pos.clone();
                SanPlus::from_move_and_play_unchecked(&mut p_copy, &shak_move).to_string()
            } else {
                uci.clone()
            };

            OpeningTreeMoveView {
                san,
                uci,
                total_games: m.total_games,
                white_pct: round_2dp((m.white_wins as f64 / m_total as f64) * 100.0),
                draw_pct: round_2dp((m.draws as f64 / m_total as f64) * 100.0),
                black_pct: round_2dp((m.black_wins as f64 / m_total as f64) * 100.0),
                white_wins: m.white_wins,
                draws: m.draws,
                black_wins: m.black_wins,
                avg_white_elo: m.avg_white_elo(),
                avg_black_elo: m.avg_black_elo(),
                last_played: None,
                sample_game_ids: Vec::new(),
            }
        })
        .collect();

    move_views.sort_unstable_by_key(|a| std::cmp::Reverse(a.total_games));

    let fen_formatted = Fen::from_position(pos.clone(), EnPassantMode::Legal).to_string();

    OpeningTreeReport {
        fen: fen_formatted,
        zobrist_hash,
        total_games: node.total_games,
        white_wins: node.white_wins,
        draws: node.draws,
        black_wins: node.black_wins,
        white_pct: if node.total_games > 0 {
            round_2dp((node.white_wins as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        draw_pct: if node.total_games > 0 {
            round_2dp((node.draws as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        black_pct: if node.total_games > 0 {
            round_2dp((node.black_wins as f64 / total as f64) * 100.0)
        } else {
            0.0
        },
        moves: move_views,
        sample_game_ids: Vec::new(),
        sample_games: Vec::new(),
    }
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
