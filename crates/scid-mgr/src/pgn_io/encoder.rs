use shakmaty::{Color, Role};

pub const ENCODE_END_GAME: u8 = 15;

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
pub fn encode_rook_like(from_file: u8, from_rank: u8, to_file: u8, to_rank: u8) -> Option<i32> {
    if to_rank == from_rank && to_file != from_file {
        Some(i32::from(to_file))
    } else if to_file == from_file && to_rank != from_rank {
        Some(8 + i32::from(to_rank))
    } else {
        None
    }
}

#[inline]
pub fn encode_scid_move_byte(
    role: Role,
    color: Color,
    from_sq: u8,
    to_sq: u8,
    promo: Option<Role>,
    is_castle_kingside: bool,
    is_castle_queenside: bool,
) -> Option<(i32, Option<u8>)> {
    if is_castle_kingside {
        return Some((10, None));
    }
    if is_castle_queenside {
        return Some((9, None));
    }

    let from_file = from_sq % 8;
    let from_rank = from_sq / 8;
    let to_file = to_sq % 8;
    let to_rank = to_sq / 8;

    let from_idx = i32::from(from_sq);
    let to_idx = i32::from(to_sq);

    match role {
        Role::Pawn => {
            let diff = if color == Color::White {
                to_idx - from_idx
            } else {
                from_idx - to_idx
            };
            if diff == 16 {
                return Some((15, None));
            }
            let group = match diff {
                7 => 0,
                8 => 1,
                9 => 2,
                _ => return None,
            };
            let row = match promo {
                None => 0,
                Some(Role::Queen) => 1,
                Some(Role::Rook) => 2,
                Some(Role::Bishop) => 3,
                Some(Role::Knight) => 4,
                Some(_) => return None,
            };
            Some((row * 3 + group, None))
        }
        Role::Knight => {
            let code = match to_idx - from_idx {
                -17 => 1,
                -15 => 2,
                -10 => 3,
                -6 => 4,
                6 => 5,
                10 => 6,
                15 => 7,
                17 => 8,
                _ => return None,
            };
            Some((code, None))
        }
        Role::Bishop => {
            let rank_diff = i32::from(to_rank) - i32::from(from_rank);
            let file_diff = i32::from(to_file) - i32::from(from_file);
            if file_diff != 0 && rank_diff == file_diff {
                Some((i32::from(to_file), None))
            } else if file_diff != 0 && rank_diff == -file_diff {
                Some((8 + i32::from(to_file), None))
            } else {
                None
            }
        }
        Role::Rook => encode_rook_like(from_file, from_rank, to_file, to_rank).map(|c| (c, None)),
        Role::Queen => {
            if let Some(code) = encode_rook_like(from_file, from_rank, to_file, to_rank) {
                return Some((code, None));
            }
            let byte2 = u8::try_from(to_idx + 64).ok()?;
            Some((i32::from(from_file), Some(byte2)))
        }
        Role::King => {
            let code = match to_idx - from_idx {
                -9 => 1,
                -8 => 2,
                -7 => 3,
                -1 => 4,
                1 => 5,
                7 => 6,
                8 => 7,
                9 => 8,
                _ => return None,
            };
            Some((code, None))
        }
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub fn update_piece_slots(
    slots: &mut [[u8; 16]; 2],
    counts: &mut [usize; 2],
    side_idx: usize,
    piece_idx: usize,
    _from_sq: u8,
    to_sq: u8,
    is_castle_kingside: bool,
    is_castle_queenside: bool,
    captured_sq: Option<u8>,
) -> bool {
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
        let rook_idx = match (0..counts[side_idx]).find(|&i| slots[side_idx][i] == rook_from) {
            Some(i) => i,
            None => return false,
        };
        slots[side_idx][rook_idx] = rook_to;
    }

    if let Some(cap_sq) = captured_sq {
        let enemy_idx = 1 - side_idx;
        let cap_idx = match (0..counts[enemy_idx]).find(|&i| slots[enemy_idx][i] == cap_sq) {
            Some(i) => i,
            None => return false,
        };
        counts[enemy_idx] -= 1;
        slots[enemy_idx][cap_idx] = slots[enemy_idx][counts[enemy_idx]];
    }

    true
}
