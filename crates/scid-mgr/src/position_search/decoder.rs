use shakmaty::{Chess, Color, Move, Position, Role, Square};
use std::collections::HashMap;

pub const ENCODE_END_GAME: u8 = 15;

/// Skips extra tags in the game blob according to SCID specification
#[inline]
pub fn skip_extra_tags(blob: &[u8], cursor: &mut usize) -> bool {
    while *cursor < blob.len() {
        let name_code = blob[*cursor];
        *cursor += 1;
        if name_code == 0 {
            return true;
        }
        if name_code == 255 {
            *cursor += 3;
            continue;
        }
        if name_code <= 240 {
            *cursor += name_code as usize;
        }
        // 241..=254 has no name to skip
        if *cursor >= blob.len() {
            return false;
        }
        let value_len = blob[*cursor] as usize;
        *cursor += 1 + value_len;
    }
    false
}

/// Decodes custom/extra tags from the SCID game blob into a HashMap
pub fn decode_extra_tags(
    blob: &[u8],
    cursor: &mut usize,
    map: &mut HashMap<String, String>,
) -> bool {
    while *cursor < blob.len() {
        let name_code = blob[*cursor];
        *cursor += 1;
        if name_code == 0 {
            return true;
        }
        if name_code == 255 {
            *cursor += 3;
            continue;
        }
        let tag_name = if name_code <= 240 {
            let name_len = name_code as usize;
            if *cursor + name_len > blob.len() {
                return false;
            }
            let name_bytes = &blob[*cursor..*cursor + name_len];
            *cursor += name_len;
            String::from_utf8_lossy(name_bytes).to_string()
        } else {
            // Standard SCID built-in extra tag codes 241..=254
            match name_code {
                241 => "EventDate".to_string(),
                242 => "Annotator".to_string(),
                243 => "Source".to_string(),
                244 => "TimeControl".to_string(),
                245 => "WhiteTitle".to_string(),
                246 => "BlackTitle".to_string(),
                247 => "WhiteType".to_string(),
                248 => "BlackType".to_string(),
                249 => "SetUp".to_string(),
                250 => "FEN".to_string(),
                _ => format!("Tag{}", name_code),
            }
        };

        if *cursor >= blob.len() {
            return false;
        }
        let value_len = blob[*cursor] as usize;
        *cursor += 1;
        if *cursor + value_len > blob.len() {
            return false;
        }
        let value_bytes = &blob[*cursor..*cursor + value_len];
        *cursor += value_len;
        let tag_val = String::from_utf8_lossy(value_bytes).to_string();
        map.insert(tag_name, tag_val);
    }
    false
}

/// Parses the initial position and advances cursor past flags and optional FEN
#[inline]
pub fn parse_start_position(blob: &[u8], cursor: &mut usize) -> Option<Chess> {
    if !skip_extra_tags(blob, cursor) || *cursor >= blob.len() {
        return None;
    }

    let flags = blob[*cursor];
    *cursor += 1;

    if flags & 0x01 != 0 {
        let fen_start = *cursor;
        while *cursor < blob.len() && blob[*cursor] != 0 {
            *cursor += 1;
        }
        let fen_bytes = &blob[fen_start..*cursor];
        if *cursor < blob.len() {
            *cursor += 1; // consume null byte
        }
        let fen_str = std::str::from_utf8(fen_bytes).ok()?;
        let fen: shakmaty::fen::Fen = fen_str.parse().ok()?;
        fen.into_position(shakmaty::CastlingMode::Standard).ok()
    } else {
        Some(Chess::default())
    }
}

/// Standard starting piece table mapping (16 slots per side)
#[inline]
pub fn standard_piece_slots() -> [[u8; 16]; 2] {
    [
        // White: 0:K(e1=4), 1:Ra1(0), 2:Nb1(1), 3:Bc1(2), 4:Qd1(3), 5:Bf1(5), 6:Ng1(6), 7:Rh1(7), 8..15: Pawns a2..h2 (8..15)
        [4, 0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        // Black: 0:K(e8=60), 1:Ra8(56), 2:Nb8(57), 3:Bc8(58), 4:Qd8(59), 5:Bf8(61), 6:Ng8(62), 7:Rh8(63), 8..15: Pawns a7..h7 (48..55)
        [
            60, 56, 57, 58, 59, 61, 62, 63, 48, 49, 50, 51, 52, 53, 54, 55,
        ],
    ]
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub fn update_slots_on_move(
    slots: &mut [[u8; 16]; 2],
    counts: &mut [usize; 2],
    side_idx: usize,
    piece_idx: usize,
    to_sq: u8,
    is_castle_kingside: bool,
    is_castle_queenside: bool,
    captured_sq: Option<u8>,
) {
    slots[side_idx][piece_idx] = to_sq;

    if is_castle_kingside || is_castle_queenside {
        let (rook_from, rook_to) = if side_idx == 0 {
            if is_castle_kingside {
                (7, 5)
            } else {
                (0, 3)
            }
        } else {
            if is_castle_kingside {
                (63, 61)
            } else {
                (56, 59)
            }
        };
        if let Some(r_idx) = (0..counts[side_idx]).find(|&i| slots[side_idx][i] == rook_from) {
            slots[side_idx][r_idx] = rook_to;
        }
    }

    if let Some(cap_sq) = captured_sq {
        let enemy_idx = 1 - side_idx;
        if let Some(cap_idx) = (0..counts[enemy_idx]).find(|&i| slots[enemy_idx][i] == cap_sq) {
            counts[enemy_idx] -= 1;
            slots[enemy_idx][cap_idx] = slots[enemy_idx][counts[enemy_idx]];
        }
    }
}

/// Decodes next move directly from raw byte stream
pub fn decode_raw_move(
    byte: u8,
    cursor: &mut usize,
    blob: &[u8],
    pos: &Chess,
    slots: &[[u8; 16]; 2],
    counts: &[usize; 2],
) -> Option<(Move, usize, u8, bool, bool, Option<u8>)> {
    let piece_idx = (byte >> 4) as usize;
    let code = (byte & 0x0F) as i32;

    let color = pos.turn();
    let side_idx = usize::from(color == Color::Black);

    if piece_idx >= counts[side_idx] {
        return None;
    }

    let from_u8 = slots[side_idx][piece_idx];
    let from_sq = Square::try_from(from_u8).ok()?;
    let piece = pos.board().piece_at(from_sq)?;

    let from_idx = i32::from(from_u8);

    let (to_sq, promo, is_castle_k, is_castle_q) = match piece.role {
        Role::Pawn => {
            const PROMO: [Option<Role>; 16] = [
                None,
                None,
                None,
                Some(Role::Queen),
                Some(Role::Queen),
                Some(Role::Queen),
                Some(Role::Rook),
                Some(Role::Rook),
                Some(Role::Rook),
                Some(Role::Bishop),
                Some(Role::Bishop),
                Some(Role::Bishop),
                Some(Role::Knight),
                Some(Role::Knight),
                Some(Role::Knight),
                None,
            ];
            const SQDIFF: [i32; 16] = [7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9, 16];
            let idx = code as usize;
            if idx >= 16 {
                return None;
            }
            let diff = SQDIFF[idx];
            let to = if color == Color::White {
                from_idx + diff
            } else {
                from_idx - diff
            };
            if !(0..64).contains(&to) {
                return None;
            }
            (Square::try_from(to as u8).ok()?, PROMO[idx], false, false)
        }
        Role::Knight => {
            const SQDIFF: [i32; 16] = [0, -17, -15, -10, -6, 6, 10, 15, 17, 0, 0, 0, 0, 0, 0, 0];
            let idx = code as usize;
            if idx >= 16 {
                return None;
            }
            let to = from_idx + SQDIFF[idx];
            if !(0..64).contains(&to) {
                return None;
            }
            (Square::try_from(to as u8).ok()?, None, false, false)
        }
        Role::Bishop => {
            let fylediff = (code & 0x07) - i32::from(from_sq.file() as u8);
            let to = if code >= 8 {
                from_idx - 7 * fylediff
            } else {
                from_idx + 9 * fylediff
            };
            if !(0..64).contains(&to) {
                return None;
            }
            (Square::try_from(to as u8).ok()?, None, false, false)
        }
        Role::Rook => {
            let to = if code < 8 {
                i32::from(from_sq.rank() as u8) * 8 + code
            } else {
                (code - 8) * 8 + i32::from(from_sq.file() as u8)
            };
            if !(0..64).contains(&to) {
                return None;
            }
            (Square::try_from(to as u8).ok()?, None, false, false)
        }
        Role::Queen => {
            if code == i32::from(from_sq.file() as u8) {
                if *cursor >= blob.len() {
                    return None;
                }
                let b2 = blob[*cursor];
                *cursor += 1;
                let to = i32::from(b2) - 64;
                if !(0..64).contains(&to) {
                    return None;
                }
                (Square::try_from(to as u8).ok()?, None, false, false)
            } else {
                let to = if code < 8 {
                    i32::from(from_sq.rank() as u8) * 8 + code
                } else {
                    (code - 8) * 8 + i32::from(from_sq.file() as u8)
                };
                if !(0..64).contains(&to) {
                    return None;
                }
                (Square::try_from(to as u8).ok()?, None, false, false)
            }
        }
        Role::King => {
            if code == 0 {
                return None;
            } // null move
            if code <= 8 {
                const SQDIFF: [i32; 9] = [0, -9, -8, -7, -1, 1, 7, 8, 9];
                let to = from_idx + SQDIFF[code as usize];
                if !(0..64).contains(&to) {
                    return None;
                }
                (Square::try_from(to as u8).ok()?, None, false, false)
            } else if code == 9 {
                let to_sq = if color == Color::White {
                    Square::C1
                } else {
                    Square::C8
                };
                (to_sq, None, false, true)
            } else if code == 10 {
                let to_sq = if color == Color::White {
                    Square::G1
                } else {
                    Square::G8
                };
                (to_sq, None, true, false)
            } else {
                return None;
            }
        }
    };

    let mv = if is_castle_k || is_castle_q {
        let (king, rook) = if color == Color::White {
            if is_castle_k {
                (Square::E1, Square::H1)
            } else {
                (Square::E1, Square::A1)
            }
        } else {
            if is_castle_k {
                (Square::E8, Square::H8)
            } else {
                (Square::E8, Square::A8)
            }
        };
        Move::Castle { king, rook }
    } else if piece.role == Role::Pawn
        && pos.board().piece_at(to_sq).is_none()
        && from_sq.file() != to_sq.file()
    {
        Move::EnPassant {
            from: from_sq,
            to: to_sq,
        }
    } else {
        let capture = pos.board().piece_at(to_sq).map(|p| p.role);
        Move::Normal {
            role: piece.role,
            from: from_sq,
            to: to_sq,
            capture,
            promotion: promo,
        }
    };

    let captured_sq = if let Move::EnPassant { from, to } = mv {
        Some(u8::from(Square::from_coords(to.file(), from.rank())))
    } else if pos.board().piece_at(to_sq).is_some() {
        Some(u8::from(to_sq))
    } else {
        None
    };

    Some((
        mv,
        piece_idx,
        u8::from(to_sq),
        is_castle_k,
        is_castle_q,
        captured_sq,
    ))
}
