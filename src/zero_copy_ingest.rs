use crate::db::ScidFormat;
use crate::pgn_utils::{FastNameTables, ImportProgress};
use anyhow::{Context, Result};
use memmap2::Mmap;
use rayon::prelude::*;
use shakmaty::san::San;
use shakmaty::{Chess, Color, Move, Position, Role, Square};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

const ENCODE_END_GAME: u8 = 15;

#[derive(Default, Debug, Clone)]
pub struct RawPgnTags {
    pub white: String,
    pub black: String,
    pub event: String,
    pub site: String,
    pub round: String,
    pub date: u32,
    pub result: u8,
    pub eco_code: u16,
    pub white_elo: u16,
    pub black_elo: u16,
    pub fen: Option<String>,
}

/// Standard starting piece table mapping (16 slots per side)
#[inline]
fn standard_piece_slots() -> [[u8; 16]; 2] {
    [
        // White: 0:K(e1=4), 1:Ra1(0), 2:Nb1(1), 3:Bc1(2), 4:Qd1(3), 5:Bf1(5), 6:Ng1(6), 7:Rh1(7), 8..15: Pawns a2..h2 (8..15)
        [4, 0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        // Black: 0:K(e8=60), 1:Ra8(56), 2:Nb8(57), 3:Bc8(58), 4:Qd8(59), 5:Bf8(61), 6:Ng8(62), 7:Rh8(63), 8..15: Pawns a7..h7 (48..55)
        [60, 56, 57, 58, 59, 61, 62, 63, 48, 49, 50, 51, 52, 53, 54, 55],
    ]
}

#[inline]
fn encode_rook_like(from_file: u8, from_rank: u8, to_file: u8, to_rank: u8) -> Option<i32> {
    if to_rank == from_rank && to_file != from_file {
        Some(i32::from(to_file))
    } else if to_file == from_file && to_rank != from_rank {
        Some(8 + i32::from(to_rank))
    } else {
        None
    }
}

#[inline]
fn encode_scid_move_byte(
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
fn update_piece_slots(
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
            if is_castle_kingside { (7, 5) } else { (0, 3) }
        } else {
            if is_castle_kingside { (63, 61) } else { (56, 59) }
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

/// Zero-copy game parser using Shakmaty Bitboard engine and direct SCID byte packing
pub fn parse_game_bytes_fast(bytes: &[u8]) -> Option<(RawPgnTags, Vec<u8>)> {
    let mut tags = RawPgnTags::default();
    let mut moves_start_idx = 0;
    let len = bytes.len();

    // 1. Tag parsing loop over bytes
    let mut cursor = 0;
    while cursor < len {
        if bytes[cursor] == b'[' {
            // Find end of line or bracket
            let line_start = cursor;
            while cursor < len && bytes[cursor] != b']' && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            if cursor < len && bytes[cursor] == b']' {
                cursor += 1; // consume ']'
                let tag_slice = &bytes[line_start + 1..cursor - 1];
                if let Some(space_pos) = tag_slice.iter().position(|&b| b == b' ') {
                    let tag_name = std::str::from_utf8(&tag_slice[..space_pos]).unwrap_or("").trim();
                    let tag_val_raw = std::str::from_utf8(&tag_slice[space_pos + 1..]).unwrap_or("").trim();
                    let val = tag_val_raw.trim_matches('"');

                    match tag_name {
                        "White" => tags.white = val.to_string(),
                        "Black" => tags.black = val.to_string(),
                        "Event" => tags.event = val.to_string(),
                        "Site" => tags.site = val.to_string(),
                        "Round" => tags.round = val.to_string(),
                        "Date" => {
                            let mut y = 0u32;
                            let mut m = 0u32;
                            let mut d = 0u32;
                            for (part, chunk) in val.split('.').enumerate() {
                                let num = chunk.parse::<u32>().unwrap_or(0);
                                match part {
                                    0 => y = num,
                                    1 => m = num,
                                    2 => d = num,
                                    _ => break,
                                }
                            }
                            tags.date = (y << 9) | (m << 5) | d;
                        }
                        "Result" => {
                            tags.result = match val {
                                "1-0" => 1,
                                "0-1" => 2,
                                "1/2-1/2" => 3,
                                _ => 0,
                            };
                        }
                        "ECO" => {
                            tags.eco_code = chess_scid_rw::eco::eco_from_string(val).unwrap_or(0);
                        }
                        "WhiteElo" => {
                            tags.white_elo = val.parse::<u16>().unwrap_or(0);
                        }
                        "BlackElo" => {
                            tags.black_elo = val.parse::<u16>().unwrap_or(0);
                        }
                        "FEN" => {
                            tags.fen = Some(val.to_string());
                        }
                        _ => {}
                    }
                }
            }
            while cursor < len && (bytes[cursor] == b'\r' || bytes[cursor] == b'\n' || bytes[cursor] == b' ') {
                cursor += 1;
            }
            moves_start_idx = cursor;
        } else if bytes[cursor] == b'\r' || bytes[cursor] == b'\n' || bytes[cursor] == b' ' || bytes[cursor] == b'\t' {
            cursor += 1;
            moves_start_idx = cursor;
        } else {
            break; // Start of moves
        }
    }

    let moves_bytes = if moves_start_idx < len {
        &bytes[moves_start_idx..]
    } else {
        &[]
    };

    // 2. Initialize Chess Bitboard State
    let mut pos: Chess = Chess::default();
    let mut slots = standard_piece_slots();
    let mut counts = [16usize, 16];

    let mut out: Vec<u8> = Vec::with_capacity(128);
    out.push(0); // (a) no extra tags
    out.push(0); // (b) standard starting position

    // 3. Ultra-fast byte-level SAN token iterator
    let moves_str = std::str::from_utf8(moves_bytes).unwrap_or("");
    let mut in_comment = false;
    let mut in_variation = 0;

    for raw_token in moves_str.split_whitespace() {
        if raw_token.starts_with('{') {
            in_comment = true;
        }
        if in_comment {
            if raw_token.ends_with('}') {
                in_comment = false;
            }
            continue;
        }

        if raw_token.starts_with('(') {
            in_variation += 1;
            continue;
        }
        if in_variation > 0 {
            if raw_token.ends_with(')') {
                in_variation -= 1;
            }
            continue;
        }

        if raw_token.starts_with('$') {
            continue;
        }

        let token = raw_token.trim_end_matches(')');
        if token.is_empty() {
            continue;
        }

        // Check for termination marker
        if token == "1-0" || token == "0-1" || token == "1/2-1/2" || token == "*" {
            break;
        }

        // Move token handling (skip "1.", "23...")
        let move_str = if token.chars().next()?.is_ascii_digit() && token.contains('.') {
            let without_num = token.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            if without_num.is_empty() {
                continue;
            }
            without_num
        } else {
            token
        };

        // Shakmaty high-speed SAN resolution
        let san: San = move_str.parse().ok()?;
        let mv = san.to_move(&pos).ok()?;

        let color = pos.turn();
        let side_idx = usize::from(color == Color::Black);

        // Extract move properties
        let (from_sq, to_sq, role, promo, is_castle_kingside, is_castle_queenside, captured_sq) = match mv {
            Move::Normal {
                role,
                from,
                to,
                capture,
                promotion,
            } => {
                let cap_sq = capture.map(|_| u8::from(to));
                (
                    u8::from(from),
                    u8::from(to),
                    role,
                    promotion,
                    false,
                    false,
                    cap_sq,
                )
            }
            Move::EnPassant { from, to } => {
                let cap_rank = from.rank();
                let cap_file = to.file();
                let cap_sq = u8::from(Square::from_coords(cap_file, cap_rank));
                (
                    u8::from(from),
                    u8::from(to),
                    Role::Pawn,
                    None,
                    false,
                    false,
                    Some(cap_sq),
                )
            }
            Move::Castle { king, rook } => {
                let is_kingside = rook.file() > king.file();
                let to_file = if is_kingside {
                    shakmaty::File::G
                } else {
                    shakmaty::File::C
                };
                let to_sq = u8::from(Square::from_coords(to_file, king.rank()));
                (
                    u8::from(king),
                    to_sq,
                    Role::King,
                    None,
                    is_kingside,
                    !is_kingside,
                    None,
                )
            }
            Move::Put { .. } => return None,
        };

        // Find piece index in SCID slots
        let piece_idx = (0..16).find(|&i| slots[side_idx][i] == from_sq)?;

        // Encode SCID move byte
        let (code, extra_byte) = encode_scid_move_byte(
            role,
            color,
            from_sq,
            to_sq,
            promo,
            is_castle_kingside,
            is_castle_queenside,
        )?;

        let raw_byte = ((piece_idx as u8) << 4) | (code as u8 & 0x0F);
        out.push(raw_byte);
        if let Some(b2) = extra_byte {
            out.push(b2);
        }

        // Update SCID piece slots
        if !update_piece_slots(
            &mut slots,
            &mut counts,
            side_idx,
            piece_idx,
            from_sq,
            to_sq,
            is_castle_kingside,
            is_castle_queenside,
            captured_sq,
        ) {
            return None;
        }

        // Update Bitboard state in-place with Shakmaty
        pos.play_unchecked(&mv);
    }

    out.push(ENCODE_END_GAME);
    Some((tags, out))
}

pub fn import_pgn_ultra_fast<F>(
    index_path: &Path,
    pgn_path: &Path,
    format: ScidFormat,
    mut progress_cb: F,
) -> Result<(usize, usize)>
where
    F: FnMut(&ImportProgress),
{
    let file = File::open(pgn_path)
        .with_context(|| format!("Failed to open PGN file: {}", pgn_path.display()))?;
    let total_bytes = file.metadata()?.len();

    // 1. Memory-Map the entire PGN file into virtual memory
    let mmap = unsafe { Mmap::map(&file)? };
    #[cfg(target_os = "linux")]
    let _ = mmap.advise(memmap2::Advice::Sequential);

    let (paths_idx, paths_names, paths_games) = match format {
        ScidFormat::Si4 => {
            let p = chess_scid_rw::Si4Paths::from_index_path(index_path);
            (p.index, p.namebase, p.games)
        }
        ScidFormat::Si5 => {
            let p = chess_scid_rw::Si5Paths::from_index_path(index_path);
            (p.index, p.namebase, p.games)
        }
    };

    // 2. SIMD Search for all [Event boundaries across the mmap buffer
    let finder = memchr::memmem::Finder::new(b"[Event ");
    let mut boundaries: Vec<usize> = Vec::with_capacity((total_bytes / 900) as usize);

    for idx in finder.find_iter(&mmap) {
        // Ensure boundary is at line start (idx == 0 or preceding char is \n)
        if idx == 0 || mmap[idx - 1] == b'\n' {
            boundaries.push(idx);
        }
    }

    if boundaries.is_empty() {
        return Ok((0, 0));
    }

    let total_games = boundaries.len();
    let mut entries: Vec<chess_scid_rw::entry::IndexEntry> = Vec::with_capacity(total_games);
    let mut names = FastNameTables::from_name_tables(&chess_scid_rw::names::NameTables::default());
    let mut games_bytes: Vec<u8> = Vec::with_capacity(total_bytes as usize / 8);

    let start_time = Instant::now();
    let mut last_progress_report = Instant::now();

    // 3. Process batches in parallel with Rayon
    const CHUNK_SIZE: usize = 16000;
    let num_chunks = total_games.div_ceil(CHUNK_SIZE);

    let mut imported = 0;
    let mut errors = 0;

    for chunk_idx in 0..num_chunks {
        let start_game = chunk_idx * CHUNK_SIZE;
        let end_game = (start_game + CHUNK_SIZE).min(total_games);

        let slice_indices: Vec<(usize, usize)> = (start_game..end_game)
            .map(|i| {
                let start_b = boundaries[i];
                let end_b = if i + 1 < total_games {
                    boundaries[i + 1]
                } else {
                    mmap.len()
                };
                (start_b, end_b)
            })
            .collect();

        // Parallel parse & encode
        let parsed_results: Vec<Option<(RawPgnTags, Vec<u8>)>> = slice_indices
            .par_iter()
            .map(|&(start_b, end_b)| parse_game_bytes_fast(&mmap[start_b..end_b]))
            .collect();

        // Fast main-thread name lookup & index packing
        for item in parsed_results {
            match item {
                Some((tags, blob)) => {
                    let length = blob.len() as u32;
                    let offset = games_bytes.len() as u64;

                    let white_id = names.player_id(&tags.white);
                    let black_id = names.player_id(&tags.black);
                    let event_id = names.event_id(&tags.event);
                    let site_id = names.site_id(&tags.site);
                    let round_id = names.round_id(&tags.round);

                    entries.push(chess_scid_rw::entry::IndexEntry {
                        offset,
                        length,
                        white_id,
                        black_id,
                        event_id,
                        site_id,
                        round_id,
                        result: tags.result,
                        eco_code: tags.eco_code,
                        date: tags.date,
                        white_elo: tags.white_elo,
                        black_elo: tags.black_elo,
                        non_standard_start: tags.fen.is_some(),
                        deleted: false,
                    });

                    games_bytes.extend_from_slice(&blob);
                    imported += 1;
                }
                None => {
                    errors += 1;
                }
            }
        }

        let processed_bytes = if end_game < total_games {
            boundaries[end_game] as u64
        } else {
            total_bytes
        };

        if last_progress_report.elapsed().as_millis() >= 100 {
            let total_elapsed_secs = start_time.elapsed().as_secs_f64();
            let speed_gps = if total_elapsed_secs > 0.01 {
                imported as f64 / total_elapsed_secs
            } else {
                0.0
            };

            let percent = if total_bytes > 0 {
                (processed_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let remaining_bytes = total_bytes.saturating_sub(processed_bytes);
            let bytes_per_sec = if total_elapsed_secs > 0.01 {
                processed_bytes as f64 / total_elapsed_secs
            } else {
                1.0
            };
            let eta_seconds = if bytes_per_sec > 10.0 {
                (remaining_bytes as f64 / bytes_per_sec) as u64
            } else {
                0
            };

            progress_cb(&ImportProgress {
                processed_bytes,
                total_bytes,
                percent,
                imported_games: imported,
                errors,
                speed_gps,
                eta_seconds,
            });
            last_progress_report = Instant::now();
        }
    }

    // 4. Save index, names, and games
    let name_tables = names.to_name_tables();
    let (index_bytes, names_bytes) = match format {
        ScidFormat::Si4 => (
            chess_scid_rw::si4::index::write_all_entries(&entries),
            chess_scid_rw::si4::namebase::write_namebase(&name_tables),
        ),
        ScidFormat::Si5 => (
            chess_scid_rw::si5::index::write_all_entries(&entries),
            chess_scid_rw::si5::namebase::write_namebase(&name_tables),
        ),
    };

    std::fs::write(&paths_idx, index_bytes).context("Writing index file")?;
    std::fs::write(&paths_names, names_bytes).context("Writing names file")?;
    std::fs::write(&paths_games, games_bytes).context("Writing games file")?;

    let total_elapsed = start_time.elapsed().as_secs_f64();
    let speed_gps = if total_elapsed > 0.0 {
        imported as f64 / total_elapsed
    } else {
        0.0
    };

    progress_cb(&ImportProgress {
        processed_bytes: total_bytes,
        total_bytes,
        percent: 100.0,
        imported_games: imported,
        errors,
        speed_gps,
        eta_seconds: 0,
    });

    Ok((imported, errors))
}
