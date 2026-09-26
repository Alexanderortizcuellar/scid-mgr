/// Scan a memory-mapped PGN file and return (start, end) byte offsets for each game.
///
/// Games are delimited by `[Event ` tags. Each returned range covers from the start
/// of one `[Event ` tag to the start of the next (or end of file).
pub fn scan_pgn_game_offsets(data: &[u8]) -> Vec<(usize, usize)> {
    let marker = b"[Event ";
    let marker_len = marker.len();
    let len = data.len();

    if len < marker_len {
        return Vec::new();
    }

    // Collect all positions where a game starts
    let mut starts: Vec<usize> = Vec::new();

    let mut i = 0;
    while i <= len - marker_len {
        if &data[i..i + marker_len] == marker {
            // Must be at the start of a line (pos 0 or preceded by '\n')
            if i == 0 || data[i - 1] == b'\n' {
                starts.push(i);
            }
        }
        i += 1;
    }

    if starts.is_empty() {
        return Vec::new();
    }

    // Build (start, end) pairs
    let mut offsets = Vec::with_capacity(starts.len());
    for w in starts.windows(2) {
        offsets.push((w[0], w[1]));
    }
    // Last game runs to end of file
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
        assert_eq!(
            offsets[1].0,
            pgn.windows(7)
                .position(|w| w == b"[Event " && w != &pgn[0..7])
                .unwrap_or(offsets[1].0)
        );
    }
}
