/// Helper to determine if a line starting with '[' is a valid PGN tag line (e.g. `[Event "..."`, `[Site "..."`, etc.)
#[inline]
pub fn is_pgn_tag_line(slice: &[u8]) -> bool {
    if slice.len() < 3 || slice[0] != b'[' || !slice[1].is_ascii_alphabetic() {
        return false;
    }
    let mut i = 2;
    while i < slice.len() && slice[i] != b'\n' {
        if slice[i] == b']' {
            return true;
        }
        i += 1;
    }
    false
}

/// Scan a memory-mapped PGN file and return (start, end) byte offsets for each game.
///
/// Games are delimited by tag header blocks (e.g. `[Event ...`, `[Site ...`, `[White ...`).
/// Each returned range covers from the start of one game to the start of the next (or end of file).
pub fn scan_pgn_game_offsets(data: &[u8]) -> Vec<(usize, usize)> {
    let len = data.len();
    if len == 0 {
        return Vec::new();
    }

    let mut starts: Vec<usize> = Vec::new();
    let mut cursor = 0;

    while cursor < len {
        // Skip leading whitespace / blank lines
        while cursor < len && data[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= len {
            break;
        }

        let game_start = cursor;
        let mut in_tags = false;

        // Check if game starts with tags
        if (cursor == 0 || data[cursor - 1] == b'\n') && data[cursor] == b'[' {
            if is_pgn_tag_line(&data[cursor..]) {
                in_tags = true;
            }
        }

        if in_tags {
            starts.push(game_start);

            // 1. Consume all consecutive tag lines
            while cursor < len {
                if (cursor == 0 || data[cursor - 1] == b'\n') && data[cursor] == b'[' {
                    if is_pgn_tag_line(&data[cursor..]) {
                        // Advance to end of this tag line
                        while cursor < len && data[cursor] != b'\n' {
                            cursor += 1;
                        }
                        if cursor < len && data[cursor] == b'\n' {
                            cursor += 1;
                        }
                        continue;
                    }
                }
                break;
            }

            // 2. Consume move text until next tag line or EOF
            while cursor < len {
                if (cursor == 0 || data[cursor - 1] == b'\n') && data[cursor] == b'[' {
                    if is_pgn_tag_line(&data[cursor..]) {
                        // Found start of next game!
                        break;
                    }
                }
                cursor += 1;
            }
        } else {
            // Headerless game or move text: advance until next tag line or EOF
            starts.push(game_start);
            while cursor < len {
                if (cursor == 0 || data[cursor - 1] == b'\n') && data[cursor] == b'[' {
                    if is_pgn_tag_line(&data[cursor..]) {
                        break;
                    }
                }
                cursor += 1;
            }
        }
    }

    if starts.is_empty() {
        return Vec::new();
    }

    let mut offsets = Vec::with_capacity(starts.len());
    for w in starts.windows(2) {
        offsets.push((w[0], w[1]));
    }
    offsets.push((*starts.last().unwrap(), len));

    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_empty() {
        assert!(scan_pgn_game_offsets(b"").is_empty());
    }

    #[test]
    fn test_scan_single_game() {
        let pgn = b"[Event \"Test\"]\n\n1. e4 e5 *\n";
        let offsets = scan_pgn_game_offsets(pgn);
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0], (0, pgn.len()));
    }

    #[test]
    fn test_scan_two_games() {
        let pgn = b"[Event \"G1\"]\n\n1. e4 *\n\n[Event \"G2\"]\n\n1. d4 *\n";
        let offsets = scan_pgn_game_offsets(pgn);
        assert_eq!(offsets.len(), 2);
        assert_eq!(offsets[0].0, 0);
        let g2_pos = pgn
            .windows(12)
            .position(|w| w == b"[Event \"G2\"]")
            .unwrap();
        assert_eq!(offsets[0].1, g2_pos);
        assert_eq!(offsets[1].0, g2_pos);
        assert_eq!(offsets[1].1, pgn.len());
    }

    #[test]
    fn test_scan_games_missing_event_header() {
        let pgn = b"[Site \"Nagykanizsa HUN\"]\n[Date \"2026.08.15\"]\n[White \"Player A\"]\n[Black \"Player B\"]\n\n1. e4 e5 1-0\n\n[Site \"Budapest HUN\"]\n[White \"Player C\"]\n[Black \"Player D\"]\n\n1. d4 d5 0-1\n";
        let offsets = scan_pgn_game_offsets(pgn);
        assert_eq!(offsets.len(), 2);
        assert_eq!(offsets[0].0, 0);
        assert_eq!(&pgn[offsets[0].0..offsets[0].0 + 6], b"[Site ");
        assert_eq!(&pgn[offsets[1].0..offsets[1].0 + 6], b"[Site ");
    }

    #[test]
    fn test_scan_games_with_bracket_in_comment() {
        let pgn = b"[Event \"G1\"]\n\n1. e4 { [note] in comment } e5 1-0\n\n[White \"Player X\"]\n\n1. c4 c5 *";
        let offsets = scan_pgn_game_offsets(pgn);
        assert_eq!(offsets.len(), 2);
        assert_eq!(offsets[0].0, 0);
        assert_eq!(&pgn[offsets[1].0..offsets[1].0 + 7], b"[White ");
    }
}
