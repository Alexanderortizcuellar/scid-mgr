use anyhow::Result;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{Chess, Color, EnPassantMode, Position, Role, Square};

use super::types::PositionTargetMatcher;

/// Parses FEN or partial board string into list of required (Square, Role, Color) pieces
pub fn parse_piece_placements(board_str: &str) -> Vec<(Square, Role, Color)> {
    let mut pieces = Vec::new();
    let board_part = board_str
        .split_whitespace()
        .next()
        .unwrap_or(board_str.trim());
    let ranks: Vec<&str> = board_part.split('/').collect();
    if ranks.len() != 8 {
        return pieces;
    }

    for (rank_idx, rank_str) in ranks.iter().enumerate() {
        let rank = 7 - rank_idx as u8;
        let mut file = 0u8;
        for ch in rank_str.chars() {
            if let Some(digit) = ch.to_digit(10) {
                file += digit as u8;
            } else {
                let color = if ch.is_uppercase() {
                    Color::White
                } else {
                    Color::Black
                };
                let role = match ch.to_ascii_lowercase() {
                    'p' => Some(Role::Pawn),
                    'n' => Some(Role::Knight),
                    'b' => Some(Role::Bishop),
                    'r' => Some(Role::Rook),
                    'q' => Some(Role::Queen),
                    'k' => Some(Role::King),
                    _ => None,
                };
                if let Some(r) = role {
                    if file < 8 {
                        if let Ok(sq) = Square::try_from(rank * 8 + file) {
                            pieces.push((sq, r, color));
                        }
                    }
                }
                file += 1;
            }
        }
    }

    pieces
}

#[inline]
pub fn matches_piece_placements(pos: &Chess, required: &[(Square, Role, Color)]) -> bool {
    let board = pos.board();
    for &(sq, role, color) in required {
        match board.piece_at(sq) {
            Some(p) if p.role == role && p.color == color => {}
            _ => return false,
        }
    }
    true
}

pub fn parse_position_matcher(
    fen_str: &str,
    turn_param: Option<&str>,
    mode_param: Option<&str>,
) -> Result<PositionTargetMatcher> {
    let trimmed = fen_str.trim();
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(anyhow::anyhow!("Empty position string"));
    }

    let board_part = tokens[0];
    let mode = mode_param.unwrap_or("auto").to_lowercase();

    // Determine turn from parameter or 2nd FEN token if explicitly provided
    let turn_token = turn_param.or_else(|| {
        if tokens.len() >= 2 && (tokens[1] == "w" || tokens[1] == "b" || tokens[1] == "any") {
            Some(tokens[1])
        } else {
            None
        }
    });

    let turn = match turn_token.map(|s| s.to_lowercase()).as_deref() {
        Some("w") | Some("white") => Some(Color::White),
        Some("b") | Some("black") => Some(Color::Black),
        _ => None, // "any", "either", or omitted
    };

    if mode == "partial" {
        let pieces = parse_piece_placements(board_part);
        if pieces.is_empty() {
            return Err(anyhow::anyhow!("No valid pieces found in: {}", fen_str));
        }
        return Ok(PositionTargetMatcher::PartialPieces(pieces));
    }

    if mode == "exact" && tokens.len() >= 4 {
        if let Ok(fen) = trimmed.parse::<shakmaty::fen::Fen>() {
            if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
                let h: Zobrist64 = pos.zobrist_hash(EnPassantMode::Legal);
                return Ok(PositionTargetMatcher::ExactHash(h.0));
            }
        }
    }

    // Try parsing as full 64-square board
    let full_fen_candidate = if tokens.len() == 1 {
        format!("{} w - - 0 1", board_part)
    } else {
        trimmed.to_string()
    };

    if let Ok(fen) = full_fen_candidate.parse::<shakmaty::fen::Fen>() {
        if let Ok(pos) = fen.into_position::<Chess>(shakmaty::CastlingMode::Standard) {
            return Ok(PositionTargetMatcher::BoardWithTurn {
                board: pos.board().clone(),
                turn,
            });
        }
    }

    // If it could not be parsed as a full legal board (e.g. missing kings, partial board)
    let pieces = parse_piece_placements(board_part);
    if !pieces.is_empty() {
        return Ok(PositionTargetMatcher::PartialPieces(pieces));
    }

    Err(anyhow::anyhow!(
        "Could not parse board position: {}",
        fen_str
    ))
}
