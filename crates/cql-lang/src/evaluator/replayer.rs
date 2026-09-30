use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::{CastlingMode, Chess, Position};
use std::collections::HashMap;
use std::str::FromStr;

use super::matcher::matches_single_ply;
use super::QueryMatchResult;
use crate::search::path::MoveRecord;
use crate::search::query::SearchQuery;

/// Zero-allocation header extraction and move-text slicing from raw PGN string
pub fn split_pgn_headers_and_moves(pgn_str: &str) -> (HashMap<String, String>, &str) {
    let mut headers = HashMap::new();
    let mut move_start = 0;
    let mut in_header = true;

    for line in pgn_str.lines() {
        let trimmed = line.trim();
        let line_offset = line.as_ptr() as usize - pgn_str.as_ptr() as usize;

        if in_header {
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let inner = &trimmed[1..trimmed.len() - 1];
                if let Some(space_idx) = inner.find(' ') {
                    let key = inner[..space_idx].trim().to_string();
                    let val = inner[space_idx + 1..].trim().trim_matches('"').to_string();
                    headers.insert(key, val);
                }
            } else if !trimmed.is_empty() {
                in_header = false;
                move_start = line_offset;
                break;
            }
        }
    }

    let moves_str = if in_header {
        ""
    } else {
        &pgn_str[move_start..]
    };
    (headers, moves_str)
}

/// Parse headers and move text from raw PGN string
pub fn parse_pgn_headers_and_moves(pgn_str: &str) -> (HashMap<String, String>, String) {
    let (headers, moves_str) = split_pgn_headers_and_moves(pgn_str);
    (headers, moves_str.to_string())
}

/// Single-pass streaming evaluation directly on PGN move text with zero vector allocations and early exit
pub fn evaluate_pgn_streaming(
    query: &SearchQuery,
    start_pos: Chess,
    moves_str: &str,
) -> QueryMatchResult {
    let mut matching_plies = Vec::new();
    let mut current_pos = start_pos;
    if matches_single_ply(query, &current_pos, 0, None) {
        matching_plies.push(0);
    }

    let max_cutoff = query.max_ply_cutoff();
    let mut ply_count = 0;
    let mut chars = moves_str.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if let Some(cutoff) = max_cutoff {
            if ply_count >= cutoff {
                break;
            }
        }

        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '{' {
            chars.next();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
            }
            continue;
        }

        if ch == '(' {
            let mut depth = 1;
            chars.next();
            for c in chars.by_ref() {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            continue;
        }

        if ch == '$' {
            chars.next();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    chars.next();
                } else {
                    break;
                }
            }
            continue;
        }

        // Read next word / token
        let mut token = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '{' || c == '(' || c == '$' {
                break;
            }
            token.push(c);
            chars.next();
        }

        let token_trim = token.trim();
        if token_trim.is_empty()
            || token_trim.ends_with('.')
            || token_trim.contains("...")
            || token_trim == "1-0"
            || token_trim == "0-1"
            || token_trim == "1/2-1/2"
            || token_trim == "*"
        {
            continue;
        }

        let san_core = token_trim.trim_end_matches(['!', '?']);
        if let Ok(san_plus) = SanPlus::from_str(san_core) {
            if let Ok(mv) = san_plus.san.to_move(&current_pos) {
                let pos_before = current_pos.clone();
                current_pos.play_unchecked(&mv);
                ply_count += 1;
                let is_check = current_pos.is_check();

                let move_rec = MoveRecord {
                    ply: ply_count,
                    mv,
                    san: token_trim.to_string(),
                    is_check,
                    nags: Vec::new(),
                    comment: None,
                };

                if matches_single_ply(
                    query,
                    &current_pos,
                    ply_count,
                    Some((&pos_before, &move_rec)),
                ) {
                    matching_plies.push(ply_count);
                }
            }
        }
    }

    let is_match = !matching_plies.is_empty();
    QueryMatchResult {
        is_match,
        match_count: matching_plies.len(),
        matching_plies,
    }
}

/// Replay game moves into sequential Chess board positions and MoveRecord timeline
pub fn replay_game(
    headers: &HashMap<String, String>,
    moves_str: &str,
) -> (Vec<Chess>, Vec<MoveRecord>) {
    let mut positions = Vec::new();
    let mut move_records: Vec<MoveRecord> = Vec::new();

    // Start position (Standard or custom FEN)
    let start_pos = if let Some(fen_str) = headers.get("FEN") {
        Fen::from_str(fen_str.trim())
            .ok()
            .and_then(|f| f.into_position::<Chess>(CastlingMode::Standard).ok())
            .unwrap_or_default()
    } else {
        Chess::default()
    };

    let mut current_pos = start_pos.clone();
    positions.push(current_pos.clone());

    let mut ply_count = 0;
    let mut chars = moves_str.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '{' {
            chars.next();
            let mut comment = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                comment.push(c);
            }
            let trimmed_comment = comment.trim().to_string();
            if !trimmed_comment.is_empty() {
                if let Some(last_move) = move_records.last_mut() {
                    last_move.comment = Some(trimmed_comment);
                }
            }
            continue;
        }

        if ch == '(' {
            // Skip recursive variation
            let mut depth = 1;
            chars.next();
            for c in chars.by_ref() {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            continue;
        }

        if ch == '$' {
            chars.next();
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Ok(nag_val) = num_str.parse::<u8>() {
                if let Some(last_move) = move_records.last_mut() {
                    last_move.nags.push(nag_val);
                }
            }
            continue;
        }

        // Read next word / token
        let mut token = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '{' || c == '(' || c == '$' {
                break;
            }
            token.push(c);
            chars.next();
        }

        let token_trim = token.trim();
        if token_trim.is_empty()
            || token_trim.ends_with('.')
            || token_trim.contains("...")
            || token_trim == "1-0"
            || token_trim == "0-1"
            || token_trim == "1/2-1/2"
            || token_trim == "*"
        {
            continue;
        }

        // Extract NAG suffixes like !, ?, !!, ??
        let mut nag_code = None;
        if token_trim.ends_with("!!") {
            nag_code = Some(3);
        } else if token_trim.ends_with("??") {
            nag_code = Some(4);
        } else if token_trim.ends_with("!?") {
            nag_code = Some(5);
        } else if token_trim.ends_with("?!") {
            nag_code = Some(6);
        } else if token_trim.ends_with('!') {
            nag_code = Some(1);
        } else if token_trim.ends_with('?') {
            nag_code = Some(2);
        }

        let san_core = token_trim.trim_end_matches(['!', '?']);
        if let Ok(san_plus) = SanPlus::from_str(san_core) {
            if let Ok(mv) = san_plus.san.to_move(&current_pos) {
                ply_count += 1;
                current_pos.play_unchecked(&mv);
                let is_check = current_pos.is_check();
                let mut nags = Vec::new();
                if let Some(n) = nag_code {
                    nags.push(n);
                }

                move_records.push(MoveRecord {
                    ply: ply_count,
                    mv: mv.clone(),
                    san: token_trim.to_string(),
                    is_check,
                    nags,
                    comment: None,
                });

                positions.push(current_pos.clone());
            }
        }
    }

    (positions, move_records)
}
