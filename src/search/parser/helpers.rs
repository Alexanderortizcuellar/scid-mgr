use shakmaty::{Color, File, Rank, Role, Square};
use std::str::FromStr;

use super::super::query::{PieceMatcher, SquareOrPiece};

/// Parse piece specifier into optional (Color, Role)
pub fn parse_piece_specifier(spec: &str) -> Option<(Option<Color>, Option<Role>)> {
    if spec.len() == 1 {
        let ch = spec.chars().next().unwrap();
        return match ch {
            'P' => Some((Some(Color::White), Some(Role::Pawn))),
            'N' => Some((Some(Color::White), Some(Role::Knight))),
            'B' => Some((Some(Color::White), Some(Role::Bishop))),
            'R' => Some((Some(Color::White), Some(Role::Rook))),
            'Q' => Some((Some(Color::White), Some(Role::Queen))),
            'K' => Some((Some(Color::White), Some(Role::King))),
            'A' => Some((Some(Color::White), None)),

            'p' => Some((Some(Color::Black), Some(Role::Pawn))),
            'n' => Some((Some(Color::Black), Some(Role::Knight))),
            'b' => Some((Some(Color::Black), Some(Role::Bishop))),
            'r' => Some((Some(Color::Black), Some(Role::Rook))),
            'q' => Some((Some(Color::Black), Some(Role::Queen))),
            'k' => Some((Some(Color::Black), Some(Role::King))),
            'a' => Some((Some(Color::Black), None)),

            _ => None,
        };
    }

    let lower = spec.to_lowercase();
    match lower.as_str() {
        "wq" | "white_queen" | "white_queens" | "white_q" => {
            Some((Some(Color::White), Some(Role::Queen)))
        }
        "wr" | "white_rook" | "white_rooks" | "white_r" => {
            Some((Some(Color::White), Some(Role::Rook)))
        }
        "wb" | "white_bishop" | "white_bishops" | "white_b" => {
            Some((Some(Color::White), Some(Role::Bishop)))
        }
        "wn" | "white_knight" | "white_knights" | "white_n" => {
            Some((Some(Color::White), Some(Role::Knight)))
        }
        "wp" | "white_pawn" | "white_pawns" | "white_p" => {
            Some((Some(Color::White), Some(Role::Pawn)))
        }
        "wk" | "white_king" | "white_kings" | "white_k" => {
            Some((Some(Color::White), Some(Role::King)))
        }

        "bq" | "black_queen" | "black_queens" | "black_q" => {
            Some((Some(Color::Black), Some(Role::Queen)))
        }
        "br" | "black_rook" | "black_rooks" | "black_r" => {
            Some((Some(Color::Black), Some(Role::Rook)))
        }
        "bb" | "black_bishop" | "black_bishops" | "black_b" => {
            Some((Some(Color::Black), Some(Role::Bishop)))
        }
        "bn" | "black_knight" | "black_knights" | "black_n" => {
            Some((Some(Color::Black), Some(Role::Knight)))
        }
        "bp" | "black_pawn" | "black_pawns" | "black_p" => {
            Some((Some(Color::Black), Some(Role::Pawn)))
        }
        "bk" | "black_king" | "black_kings" | "black_k" => {
            Some((Some(Color::Black), Some(Role::King)))
        }

        "queen" | "queens" => Some((None, Some(Role::Queen))),
        "rook" | "rooks" => Some((None, Some(Role::Rook))),
        "bishop" | "bishops" => Some((None, Some(Role::Bishop))),
        "knight" | "knights" => Some((None, Some(Role::Knight))),
        "pawn" | "pawns" => Some((None, Some(Role::Pawn))),
        "king" | "kings" => Some((None, Some(Role::King))),

        "white" | "white_pieces" | "white_piece" => Some((Some(Color::White), None)),
        "black" | "black_pieces" | "black_piece" => Some((Some(Color::Black), None)),
        "any" | "any_piece" | "piece" | "pieces" | "occupied" => Some((None, None)),

        _ => None,
    }
}

/// Parse compact piece placement like "Kd4", "qd4", "Bf4", "pe4", "_d4", "Ae4", "ae4", "wqd4", "bke8"
pub fn parse_compact_piece_placement(
    s: &str,
) -> Option<(Square, crate::search::query::SquareContent)> {
    use crate::search::query::SquareContent;
    if s.len() == 3 {
        let (p_str, sq_str) = s.split_at(1);
        let sq = Square::from_str(&sq_str.to_lowercase()).ok()?;
        let ch = p_str.chars().next()?;
        let content = match ch {
            '_' => SquareContent::Empty,
            'K' => SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: Role::King,
            }),
            'Q' => SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: Role::Queen,
            }),
            'R' => SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: Role::Rook,
            }),
            'B' => SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: Role::Bishop,
            }),
            'N' => SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: Role::Knight,
            }),
            'P' => SquareContent::Piece(shakmaty::Piece {
                color: Color::White,
                role: Role::Pawn,
            }),
            'A' => SquareContent::Color(Color::White),
            'k' => SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: Role::King,
            }),
            'q' => SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: Role::Queen,
            }),
            'r' => SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: Role::Rook,
            }),
            'b' => SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: Role::Bishop,
            }),
            'n' => SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: Role::Knight,
            }),
            'p' => SquareContent::Piece(shakmaty::Piece {
                color: Color::Black,
                role: Role::Pawn,
            }),
            'a' => SquareContent::Color(Color::Black),
            _ => return None,
        };
        return Some((sq, content));
    }
    if s.len() == 4 {
        let (p_str, sq_str) = s.split_at(2);
        if let Ok(sq) = Square::from_str(&sq_str.to_lowercase()) {
            if let Some((color_opt, role_opt)) = parse_piece_specifier(p_str) {
                let content = match (color_opt, role_opt) {
                    (Some(c), Some(r)) => {
                        SquareContent::Piece(shakmaty::Piece { color: c, role: r })
                    }
                    (None, Some(r)) => SquareContent::Role(r),
                    (Some(c), None) => SquareContent::Color(c),
                    (None, None) => SquareContent::Occupied,
                };
                return Some((sq, content));
            }
        }
    }
    None
}

/// Parse a Square, Piece, Variable, or Empty
pub fn parse_square_or_piece(s: &str) -> Option<SquareOrPiece> {
    if s.starts_with('$') {
        return Some(SquareOrPiece::Variable(s.to_string()));
    }
    let lower = s.to_lowercase();
    if lower == "empty" || lower == "_" {
        return Some(SquareOrPiece::Empty);
    }
    if let Ok(sq) = Square::from_str(&lower) {
        return Some(SquareOrPiece::Square(sq));
    }
    if let Some((c, r)) = parse_piece_specifier(s) {
        return Some(SquareOrPiece::Piece(PieceMatcher::new(c, r)));
    }
    None
}

/// Expand a rectangular area between two squares (inclusive of all files and ranks between them)
pub fn expand_rectangular_range(from: Square, to: Square) -> Vec<Square> {
    let min_f = (from.file() as u32).min(to.file() as u32);
    let max_f = (from.file() as u32).max(to.file() as u32);
    let min_r = (from.rank() as u32).min(to.rank() as u32);
    let max_r = (from.rank() as u32).max(to.rank() as u32);

    let mut squares = Vec::with_capacity(((max_f - min_f + 1) * (max_r - min_r + 1)) as usize);
    for r in min_r..=max_r {
        for f in min_f..=max_f {
            squares.push(Square::from_coords(File::new(f), Rank::new(r)));
        }
    }
    squares
}

/// Expand a collinear ray or diagonal line between two squares
pub fn expand_diagonal_ray(from: Square, to: Square) -> Option<Vec<Square>> {
    let f1 = from.file() as i32;
    let r1 = from.rank() as i32;
    let f2 = to.file() as i32;
    let r2 = to.rank() as i32;

    let df = f2 - f1;
    let dr = r2 - r1;

    // Must be either diagonal (|df| == |dr|), horizontal (dr == 0), or vertical (df == 0)
    if df.abs() != dr.abs() && df != 0 && dr != 0 {
        return None;
    }

    let step_f = df.signum();
    let step_r = dr.signum();
    let num_steps = df.abs().max(dr.abs());

    let mut squares = Vec::with_capacity((num_steps + 1) as usize);
    for i in 0..=num_steps {
        let cur_f = (f1 + i * step_f) as u32;
        let cur_r = (r1 + i * step_r) as u32;
        squares.push(Square::from_coords(File::new(cur_f), Rank::new(cur_r)));
    }
    Some(squares)
}

/// Parse and expand a square specifier string (e.g. "e4", "a1-h2", "a1..h8", "a-h1-2", "a1-8", "diag:a1-h8")
pub fn expand_square_specifier(spec: &str) -> Option<Vec<Square>> {
    let clean = spec.trim().to_lowercase();

    // Check for explicit diagonal / ray prefix
    if let Some(rest) = clean
        .strip_prefix("diag:")
        .or_else(|| clean.strip_prefix("diagonal:"))
        .or_else(|| clean.strip_prefix("ray:"))
    {
        let (p1, p2) = if let Some((a, b)) = rest.split_once("..") {
            (a, b)
        } else if let Some((a, b)) = rest.split_once('-') {
            (a, b)
        } else if let Some((a, b)) = rest.split_once(',') {
            (a, b)
        } else {
            return None;
        };
        let sq1 = Square::from_str(p1.trim()).ok()?;
        let sq2 = Square::from_str(p2.trim()).ok()?;
        return expand_diagonal_ray(sq1, sq2);
    }

    // Check for ".." ray/diagonal notation: e.g. "a1..h8"
    if let Some((p1, p2)) = clean.split_once("..") {
        let sq1 = Square::from_str(p1.trim()).ok()?;
        let sq2 = Square::from_str(p2.trim()).ok()?;
        return expand_diagonal_ray(sq1, sq2);
    }

    // Check for light / dark whole-board square sets
    if clean == "light"
        || clean == "light_squares"
        || clean == "lightsquares"
        || clean == "light_sq"
    {
        return Some(shakmaty::Bitboard::LIGHT_SQUARES.into_iter().collect());
    }
    if clean == "dark" || clean == "dark_squares" || clean == "darksquares" || clean == "dark_sq" {
        return Some(shakmaty::Bitboard::DARK_SQUARES.into_iter().collect());
    }

    // Check for standard square name: "e4"
    if let Ok(sq) = Square::from_str(&clean) {
        return Some(vec![sq]);
    }

    // Check for "-" range notation
    if let Some((p1, p2)) = clean.split_once('-') {
        // Case A: Full squares on both sides, e.g. "a1-h2", "c3-f6", "a1-a8"
        if let (Ok(sq1), Ok(sq2)) = (Square::from_str(p1.trim()), Square::from_str(p2.trim())) {
            return Some(expand_rectangular_range(sq1, sq2));
        }

        // Case B: CQL file-rank range formats:
        // "a-h1-2" (files a..h, ranks 1..2)
        // "a-c1-3" (files a..c, ranks 1..3)
        // "a-h1" (files a..h, rank 1)
        // "a1-8" (file a, ranks 1..8)
        let chars: Vec<char> = clean.chars().collect();
        // Pattern "a-h1-2": len == 6, [0]='a', [1]='-', [2]='h', [3]='1', [4]='-', [5]='2'
        if chars.len() == 6 && chars[1] == '-' && chars[4] == '-' {
            let f1 = file_char_to_idx(chars[0])?;
            let f2 = file_char_to_idx(chars[2])?;
            let r1 = rank_char_to_idx(chars[3])?;
            let r2 = rank_char_to_idx(chars[5])?;
            let min_f = f1.min(f2);
            let max_f = f1.max(f2);
            let min_r = r1.min(r2);
            let max_r = r1.max(r2);
            let mut squares = Vec::new();
            for r in min_r..=max_r {
                for f in min_f..=max_f {
                    squares.push(Square::from_coords(File::new(f), Rank::new(r)));
                }
            }
            return Some(squares);
        }

        // Pattern "a1-8": file 'a', rank '1' to '8'
        if chars.len() == 4 && chars[2] == '-' {
            let f = file_char_to_idx(chars[0])?;
            let r1 = rank_char_to_idx(chars[1])?;
            let r2 = rank_char_to_idx(chars[3])?;
            let min_r = r1.min(r2);
            let max_r = r1.max(r2);
            let mut squares = Vec::new();
            for r in min_r..=max_r {
                squares.push(Square::from_coords(File::new(f), Rank::new(r)));
            }
            return Some(squares);
        }

        // Pattern "a-h1": file 'a' to 'h', rank '1'
        if chars.len() == 4 && chars[1] == '-' {
            let f1 = file_char_to_idx(chars[0])?;
            let f2 = file_char_to_idx(chars[2])?;
            let r = rank_char_to_idx(chars[3])?;
            let min_f = f1.min(f2);
            let max_f = f1.max(f2);
            let mut squares = Vec::new();
            for f in min_f..=max_f {
                squares.push(Square::from_coords(File::new(f), Rank::new(r)));
            }
            return Some(squares);
        }
    }

    None
}

fn file_char_to_idx(c: char) -> Option<u32> {
    match c {
        'a'..='h' => Some((c as u32) - ('a' as u32)),
        _ => None,
    }
}

fn rank_char_to_idx(c: char) -> Option<u32> {
    match c {
        '1'..='8' => Some((c as u32) - ('1' as u32)),
        _ => None,
    }
}

/// Parse source piece / square / set into a MovePattern
pub fn parse_source_part(lhs: &str, pat: &mut crate::search::query::MovePattern) {
    use crate::search::query::SquareContent;
    let clean = lhs.trim();
    if clean.is_empty() || clean == "--" || clean == "*" || clean == "_" {
        return;
    }

    if clean == "A" || clean.eq_ignore_ascii_case("w") || clean.eq_ignore_ascii_case("white") {
        pat.color = Some(Color::White);
        pat.from_pieces = Some(vec![SquareContent::Color(Color::White)]);
        return;
    }
    if clean == "a" || clean.eq_ignore_ascii_case("black") {
        pat.color = Some(Color::Black);
        pat.from_pieces = Some(vec![SquareContent::Color(Color::Black)]);
        return;
    }

    // Single piece letter (e.g. 'P', 'p', 'N', 'n', 'B', 'b', 'R', 'r', 'Q', 'q', 'K', 'k')
    if clean.len() == 1 {
        let ch = clean.chars().next().unwrap();
        if let Some((color, role)) = parse_piece_specifier(clean) {
            pat.color = color;
            pat.role = role;
            if let (Some(c), Some(r)) = (color, role) {
                pat.from_pieces = Some(vec![SquareContent::Piece(shakmaty::Piece {
                    color: c,
                    role: r,
                })]);
            } else if let Some(c) = color {
                pat.from_pieces = Some(vec![SquareContent::Color(c)]);
            } else if let Some(r) = role {
                pat.from_pieces = Some(vec![SquareContent::Role(r)]);
            }
            return;
        }

        // Single file letter for pawn moves: 'a'..='h'
        if let Some(f_idx) = file_char_to_idx(ch) {
            let file = File::new(f_idx);
            let sqs: Vec<Square> = (0..8)
                .map(|r| Square::from_coords(file, Rank::new(r)))
                .collect();
            pat.from_squares = Some(sqs);
            pat.role = Some(Role::Pawn);
            return;
        }
    }

    // 2-character square (e.g. "e7", "h4")
    if clean.len() == 2 {
        if let Ok(sq) = Square::from_str(&clean.to_lowercase()) {
            pat.from = Some(sq);
            return;
        }
    }

    // 3-character piece + square (e.g. "Pe7", "pe7", "Bc2", "Nf3", "rd8")
    if clean.len() == 3 {
        let (p_str, sq_str) = clean.split_at(1);
        if let Ok(sq) = Square::from_str(&sq_str.to_lowercase()) {
            pat.from = Some(sq);
            if let Some((color, role)) = parse_piece_specifier(p_str) {
                pat.color = color;
                pat.role = role;
                if let (Some(c), Some(r)) = (color, role) {
                    pat.from_pieces = Some(vec![SquareContent::Piece(shakmaty::Piece {
                        color: c,
                        role: r,
                    })]);
                }
            }
            return;
        }
    }

    // 4-character piece + square (e.g. "wpe7", "bpe7", "wbc2")
    if clean.len() == 4 {
        let (p_str, sq_str) = clean.split_at(2);
        if let Ok(sq) = Square::from_str(&sq_str.to_lowercase()) {
            pat.from = Some(sq);
            if let Some((color, role)) = parse_piece_specifier(p_str) {
                pat.color = color;
                pat.role = role;
                if let (Some(c), Some(r)) = (color, role) {
                    pat.from_pieces = Some(vec![SquareContent::Piece(shakmaty::Piece {
                        color: c,
                        role: r,
                    })]);
                }
            }
            return;
        }
    }

    // Bracketed square specifier: e.g. "[c3,d5]" or "[a1-h8]"
    if clean.starts_with('[') && clean.ends_with(']') {
        if let Some(sqs) = expand_square_specifier(clean) {
            pat.from_squares = Some(sqs);
        }
    }
}

/// Parse target destination square / piece / set into a MovePattern
pub fn parse_target_part(rhs: &str, pat: &mut crate::search::query::MovePattern) {
    use crate::search::query::SquareContent;
    let clean = rhs.trim();
    if clean.is_empty() || clean == "--" || clean == "*" || clean == "_" {
        return;
    }

    // Bracketed pieces or square sets: e.g. "[q,r]", "[q,r]h7", "[e4,d5]"
    if clean.starts_with('[') {
        if let Some(sqs) = expand_square_specifier(clean) {
            pat.to_squares = Some(sqs);
            return;
        }
        if let Some(close_idx) = clean.find(']') {
            let pieces_inner = &clean[1..close_idx];
            let after_bracket = &clean[close_idx + 1..];

            let mut target_pieces = Vec::new();
            for part in pieces_inner.split(',') {
                let trimmed = part.trim();
                if let Some((t_color, t_role)) = parse_piece_specifier(trimmed) {
                    let content = match (t_color, t_role) {
                        (Some(c), Some(r)) => {
                            SquareContent::Piece(shakmaty::Piece { color: c, role: r })
                        }
                        (Some(c), None) => SquareContent::Color(c),
                        (None, Some(r)) => SquareContent::Role(r),
                        (None, None) => SquareContent::Occupied,
                    };
                    target_pieces.push(content);
                }
            }

            if !target_pieces.is_empty() {
                pat.to_pieces = Some(target_pieces);
            }

            if !after_bracket.is_empty() {
                if let Ok(sq) = Square::from_str(&after_bracket.to_lowercase()) {
                    pat.to = Some(sq);
                }
            }
            return;
        }
    }

    // 2-character square (e.g. "h7", "d8", "g5")
    if clean.len() == 2 {
        if let Ok(sq) = Square::from_str(&clean.to_lowercase()) {
            pat.to = Some(sq);
            return;
        }
    }

    // 3-character piece + square (e.g. "ph7", "Ph7", "qd8", "rd5")
    if clean.len() == 3 {
        let (p_str, sq_str) = clean.split_at(1);
        if let Ok(sq) = Square::from_str(&sq_str.to_lowercase()) {
            pat.to = Some(sq);
            if let Some((t_color, t_role)) = parse_piece_specifier(p_str) {
                let content = match (t_color, t_role) {
                    (Some(c), Some(r)) => {
                        SquareContent::Piece(shakmaty::Piece { color: c, role: r })
                    }
                    (Some(c), None) => SquareContent::Color(c),
                    (None, Some(r)) => SquareContent::Role(r),
                    (None, None) => SquareContent::Occupied,
                };
                pat.to_pieces = Some(vec![content]);
                return;
            }
        }
    }

    // Single piece or target specifier (e.g. 'r', 'a', 'A', 'p', 'P', 'q', 'N')
    if let Some((t_color, t_role)) = parse_piece_specifier(clean) {
        let content = match (t_color, t_role) {
            (Some(c), Some(r)) => SquareContent::Piece(shakmaty::Piece { color: c, role: r }),
            (Some(c), None) => SquareContent::Color(c),
            (None, Some(r)) => SquareContent::Role(r),
            (None, None) => SquareContent::Occupied,
        };
        pat.to_pieces = Some(vec![content]);
        return;
    }

    // General square range or specifier
    if let Some(sqs) = expand_square_specifier(clean) {
        pat.to_squares = Some(sqs);
    }
}

fn parse_capture_move(clean: &str, pat: &mut crate::search::query::MovePattern) {
    pat.is_capture = Some(true);
    let (lhs, rhs) = if let Some(idx) = clean.to_lowercase().find("[x]") {
        (&clean[..idx], &clean[idx + 3..])
    } else if let Some((l, r)) = clean.split_once('x') {
        (l, r)
    } else if let Some((l, r)) = clean.split_once('X') {
        (l, r)
    } else {
        (clean, "")
    };

    parse_source_part(lhs, pat);
    parse_target_part(rhs, pat);
}

fn parse_quiet_move(clean: &str, pat: &mut crate::search::query::MovePattern) {
    let (lhs, rhs) = if let Some((l, r)) = clean.split_once("--") {
        (l, r)
    } else if let Some((l, r)) = clean.split_once("->") {
        (l, r)
    } else {
        (clean, "")
    };

    parse_source_part(lhs, pat);
    parse_target_part(rhs, pat);
}

/// Parse a single move pattern token inside a path/line expression
/// Supports:
/// - Standard SAN: "e4", "Nf3", "Bxh7+", "O-O", "O-O-O", "exd5"
/// - Move separator `--`: "Nf3--g5", "h4--h5", "Ph6--h7", "P--h7", "b--g4", "N--[e4,d5]"
/// - Captures `x` / `[x]`: "Pe7xd8", "P[x]d8", "P[x]a", "Bc2xh7", "Bxh7", "bxh7", "Bxph7", "Pxr", "pxN"
/// - Wildcards: "--" or "*" (any move), "A--" / "w--" (any white move), "a--" / "b--" (any black move)
/// - Piece Wildcards: "R--" (white rook moves anywhere), "r--" (black rook moves anywhere), "P--", "p--"
/// - Promotion Wildcards & Targets: "A--=Q", "P--=Q", "Pe7xd8=Q", "P[x]d8=Q", "P[x]a=Q", "Pxr=Q", "--=R", "--=\"RBN\""
pub fn parse_path_move_token(s: &str) -> Option<crate::search::query::MovePattern> {
    use crate::search::query::{MovePattern, SquareContent};
    use shakmaty::Role;

    let clean = s.trim().trim_end_matches(['+', '#', '!', '?']);
    if clean.is_empty() {
        return None;
    }

    // 1. Check for Castling moves
    if clean.eq_ignore_ascii_case("o-o") {
        return Some(MovePattern {
            san: Some("O-O".to_string()),
            role: Some(Role::King),
            ..Default::default()
        });
    }
    if clean.eq_ignore_ascii_case("o-o-o") {
        return Some(MovePattern {
            san: Some("O-O-O".to_string()),
            role: Some(Role::King),
            ..Default::default()
        });
    }

    // 2. Pure Wildcard: "--", "*", or "_" (any move / any piece moves)
    if clean == "--" || clean == "*" || clean == "_" {
        return Some(MovePattern::default());
    }

    // 3. Promotions (with or without capture, source square, or wildcard)
    if let Some((move_body, promo_part)) = clean.split_once('=') {
        let mut pat = MovePattern::default();
        parse_promotion_into_pattern(&mut pat, promo_part);

        let body = move_body.trim();
        if body.is_empty() || body == "--" || body == "*" || body == "_" {
            // Any promotion to promo_part
        } else if body.to_lowercase().contains("[x]") || body.contains('x') || body.contains('X') {
            parse_capture_move(body, &mut pat);
        } else if body.contains("--") || body.contains("->") {
            parse_quiet_move(body, &mut pat);
        } else if body == "A"
            || body.eq_ignore_ascii_case("w")
            || body.eq_ignore_ascii_case("white")
        {
            pat.color = Some(Color::White);
        } else if body == "a" || body.eq_ignore_ascii_case("black") {
            pat.color = Some(Color::Black);
        } else {
            parse_source_part(body, &mut pat);
        }

        pat.role = pat.role.or(Some(Role::Pawn));
        return Some(pat);
    }

    // 4. Capture formats: e.g. "Pe7xd8", "P[x]d8", "P[x]a", "Bc2xh7", "Bxh7", "bxh7", "Bxph7", "Pxr", "pxN", "exd5"
    if clean.to_lowercase().contains("[x]") || clean.contains('x') || clean.contains('X') {
        let mut pat = MovePattern::default();
        parse_capture_move(clean, &mut pat);
        return Some(pat);
    }

    // 5. Move Separator `--` or `->` notation: "Ph6--h7", "P--h7", "Nf3--g5", "h4--h5", "b--g4", "N--[e4,d5]"
    if clean.contains("--") || clean.contains("->") {
        let mut pat = MovePattern::default();
        parse_quiet_move(clean, &mut pat);
        return Some(pat);
    }

    // 6. Bare 2-character square: "h7", "e4", "d5" -> any pawn move to this square
    if clean.len() == 2 {
        if let Ok(sq) = Square::from_str(&clean.to_lowercase()) {
            return Some(MovePattern {
                to: Some(sq),
                role: Some(Role::Pawn),
                ..Default::default()
            });
        }
    }

    // 7. Bare 3-character piece + square: "Ph7", "ph7", "Nf3", "nf3", "Kd4", "kd4"
    if clean.len() == 3 {
        let (p_str, sq_str) = clean.split_at(1);
        if let Ok(sq) = Square::from_str(&sq_str.to_lowercase()) {
            if let Some((color, role)) = parse_piece_specifier(p_str) {
                let mut pat = MovePattern {
                    to: Some(sq),
                    color,
                    role,
                    ..Default::default()
                };
                if let (Some(c), Some(r)) = (color, role) {
                    pat.from_pieces = Some(vec![SquareContent::Piece(shakmaty::Piece {
                        color: c,
                        role: r,
                    })]);
                } else if let Some(c) = color {
                    pat.from_pieces = Some(vec![SquareContent::Color(c)]);
                }
                return Some(pat);
            }
        }
    }

    // 8. General fallback: try source part or standard SAN
    let mut pat = MovePattern::default();
    parse_source_part(clean, &mut pat);
    if pat != MovePattern::default() {
        return Some(pat);
    }

    Some(MovePattern {
        san: Some(clean.to_string()),
        ..Default::default()
    })
}

fn parse_promotion_into_pattern(pat: &mut crate::search::query::MovePattern, promo_str: &str) {
    use shakmaty::Role;
    let clean = promo_str.trim().trim_matches(['"', '[', ']', '\'']);
    if clean.len() == 1 {
        let ch = clean.chars().next().unwrap();
        match ch.to_ascii_uppercase() {
            'Q' => pat.promotion = Some(Role::Queen),
            'R' => pat.promotion = Some(Role::Rook),
            'B' => pat.promotion = Some(Role::Bishop),
            'N' => pat.promotion = Some(Role::Knight),
            _ => {}
        }
    } else {
        let mut roles = Vec::new();
        for ch in clean.chars() {
            match ch.to_ascii_uppercase() {
                'Q' => roles.push(Role::Queen),
                'R' => roles.push(Role::Rook),
                'B' => roles.push(Role::Bishop),
                'N' => roles.push(Role::Knight),
                _ => {}
            }
        }
        if !roles.is_empty() {
            if roles.len() == 1 {
                pat.promotion = Some(roles[0]);
            } else {
                pat.promotions = Some(roles);
            }
        }
    }
}
