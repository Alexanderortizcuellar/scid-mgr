use shakmaty::{Color, Piece, Square};
use std::collections::HashMap;
use std::str::FromStr;

use super::helpers::{expand_diagonal_ray, expand_square_specifier, parse_piece_specifier};
use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{PositionPattern, SearchQuery, SquareContent};

impl<'a> QueryParser<'a> {
    /// Parse a single square, a bracketed list of squares/ranges, a diagonal/ray, or a range expression
    pub(crate) fn parse_square_set(&mut self) -> Result<Vec<Square>, ParseError> {
        let pos = self.current_pos();

        // Check for diag / diagonal / ray keyword prefix: diag[a1-h8] or diag(a1, h8)
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_lower = id.to_lowercase();
            if id_lower == "diag" || id_lower == "diagonal" || id_lower == "ray" {
                self.advance();
                return self.parse_diagonal_expression();
            }
        }

        // Bracketed list: [sq1, a1-h2, a1..h8, ...]
        if let Some(Token::LBracket) = self.peek() {
            self.advance();
            let mut squares = Vec::new();
            while let Some(tok) = self.peek() {
                if let Token::RBracket = tok {
                    self.advance();
                    break;
                }
                if let Token::Comma = tok {
                    self.advance();
                    continue;
                }
                if let Some(Token::Ident(ref id)) = self.peek() {
                    let id_lower = id.to_lowercase();
                    if id_lower == "diag" || id_lower == "diagonal" || id_lower == "ray" {
                        self.advance();
                        let diag_sqs = self.parse_diagonal_expression()?;
                        squares.extend(diag_sqs);
                        continue;
                    }
                }
                let first_tok = self.expect_ident()?;
                if let Some(Token::DotDot) = self.peek() {
                    self.advance();
                    let second_tok = self.expect_ident()?;
                    let sq1 = Square::from_str(&first_tok.to_lowercase()).map_err(|_| {
                        ParseError::new(
                            format!("Invalid start square in range: {}", first_tok),
                            pos,
                        )
                    })?;
                    let sq2 = Square::from_str(&second_tok.to_lowercase()).map_err(|_| {
                        ParseError::new(format!("Invalid end square in range: {}", second_tok), pos)
                    })?;
                    let ray_sqs = expand_diagonal_ray(sq1, sq2).ok_or_else(|| {
                        ParseError::new(
                            format!(
                                "Squares {} and {} do not form a diagonal or straight line",
                                first_tok, second_tok
                            ),
                            pos,
                        )
                    })?;
                    squares.extend(ray_sqs);
                } else if let Some(sqs) = expand_square_specifier(&first_tok) {
                    squares.extend(sqs);
                } else {
                    return Err(ParseError::new(
                        format!("Invalid square or range: {}", first_tok),
                        pos,
                    ));
                }
            }
            if squares.is_empty() {
                return Err(ParseError::new(
                    "Square list cannot be empty".to_string(),
                    pos,
                ));
            }
            return Ok(squares);
        }

        // Single ident or range without brackets: e.g. "a1", "a1-h2", "a1..h8", "a-h1-2"
        let first_tok = self.expect_ident()?;
        if let Some(Token::DotDot) = self.peek() {
            self.advance();
            let second_tok = self.expect_ident()?;
            let sq1 = Square::from_str(&first_tok.to_lowercase()).map_err(|_| {
                ParseError::new(format!("Invalid start square in range: {}", first_tok), pos)
            })?;
            let sq2 = Square::from_str(&second_tok.to_lowercase()).map_err(|_| {
                ParseError::new(format!("Invalid end square in range: {}", second_tok), pos)
            })?;
            let ray_sqs = expand_diagonal_ray(sq1, sq2).ok_or_else(|| {
                ParseError::new(
                    format!(
                        "Squares {} and {} do not form a diagonal or straight line",
                        first_tok, second_tok
                    ),
                    pos,
                )
            })?;
            return Ok(ray_sqs);
        }

        if let Some(sqs) = expand_square_specifier(&first_tok) {
            Ok(sqs)
        } else {
            Err(ParseError::new(
                format!("Invalid square name or range: {}", first_tok),
                pos,
            ))
        }
    }

    /// Parse a diagonal or ray expression (e.g. diag(a1, h8), diag[a1-h8], diag a1-h8)
    pub(crate) fn parse_diagonal_expression(&mut self) -> Result<Vec<Square>, ParseError> {
        let pos = self.current_pos();
        let is_bracket = matches!(self.peek(), Some(Token::LBracket));
        let is_paren = matches!(self.peek(), Some(Token::LParen));

        if is_bracket || is_paren {
            self.advance();
            let first_tok = self.expect_ident()?;
            let (sq1, sq2) = if let Some(Token::Comma) | Some(Token::DotDot) = self.peek() {
                self.advance();
                let second_tok = self.expect_ident()?;
                let s1 = Square::from_str(&first_tok.to_lowercase()).map_err(|_| {
                    ParseError::new(format!("Invalid start square: {}", first_tok), pos)
                })?;
                let s2 = Square::from_str(&second_tok.to_lowercase()).map_err(|_| {
                    ParseError::new(format!("Invalid end square: {}", second_tok), pos)
                })?;
                (s1, s2)
            } else if let Some((p1, p2)) = first_tok.split_once('-') {
                let s1 = Square::from_str(p1.trim().to_lowercase().as_str())
                    .map_err(|_| ParseError::new(format!("Invalid start square: {}", p1), pos))?;
                let s2 = Square::from_str(p2.trim().to_lowercase().as_str())
                    .map_err(|_| ParseError::new(format!("Invalid end square: {}", p2), pos))?;
                (s1, s2)
            } else {
                return Err(ParseError::new(
                    format!("Expected second square in diagonal: {}", first_tok),
                    pos,
                ));
            };

            if is_bracket {
                self.expect_token(Token::RBracket)?;
            } else {
                self.expect_token(Token::RParen)?;
            }

            expand_diagonal_ray(sq1, sq2).ok_or_else(|| {
                ParseError::new(
                    format!(
                        "Squares {:?} and {:?} do not form a diagonal or ray",
                        sq1, sq2
                    ),
                    pos,
                )
            })
        } else {
            let first_tok = self.expect_ident()?;
            if let Some(Token::DotDot) = self.peek() {
                self.advance();
                let second_tok = self.expect_ident()?;
                let sq1 = Square::from_str(&first_tok.to_lowercase())
                    .map_err(|_| ParseError::new(format!("Invalid square: {}", first_tok), pos))?;
                let sq2 = Square::from_str(&second_tok.to_lowercase())
                    .map_err(|_| ParseError::new(format!("Invalid square: {}", second_tok), pos))?;
                expand_diagonal_ray(sq1, sq2).ok_or_else(|| {
                    ParseError::new(
                        format!(
                            "Squares {} and {} do not form a diagonal",
                            first_tok, second_tok
                        ),
                        pos,
                    )
                })
            } else if let Some((p1, p2)) = first_tok.split_once('-') {
                let sq1 = Square::from_str(p1.trim().to_lowercase().as_str())
                    .map_err(|_| ParseError::new(format!("Invalid square: {}", p1), pos))?;
                let sq2 = Square::from_str(p2.trim().to_lowercase().as_str())
                    .map_err(|_| ParseError::new(format!("Invalid square: {}", p2), pos))?;
                expand_diagonal_ray(sq1, sq2).ok_or_else(|| {
                    ParseError::new(
                        format!("Squares {} and {} do not form a diagonal", p1, p2),
                        pos,
                    )
                })
            } else {
                Err(ParseError::new(
                    format!("Expected diagonal range after diag keyword: {}", first_tok),
                    pos,
                ))
            }
        }
    }

    pub(crate) fn parse_color_modified_piece(
        &mut self,
        is_light: bool,
    ) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        let modifier_squares: Vec<Square> = if is_light {
            shakmaty::Bitboard::LIGHT_SQUARES.into_iter().collect()
        } else {
            shakmaty::Bitboard::DARK_SQUARES.into_iter().collect()
        };

        let content = if let Some(Token::LBracket) = self.peek() {
            self.advance();
            let mut pieces = Vec::new();
            while let Some(tok) = self.peek() {
                if let Token::RBracket = tok {
                    self.advance();
                    break;
                }
                if let Token::Comma = tok {
                    self.advance();
                    continue;
                }
                let pt = self.expect_ident()?;
                if pt.len() > 1
                    && !pt.starts_with("white_")
                    && !pt.starts_with("black_")
                    && parse_piece_specifier(&pt).is_none()
                {
                    let mut all_chars_valid = true;
                    let mut char_pieces = Vec::new();
                    for ch in pt.chars() {
                        let ch_str = ch.to_string();
                        if let Some((color_opt, role_opt)) = parse_piece_specifier(&ch_str) {
                            let roles = match role_opt {
                                Some(r) => vec![r],
                                None => vec![
                                    shakmaty::Role::Pawn,
                                    shakmaty::Role::Knight,
                                    shakmaty::Role::Bishop,
                                    shakmaty::Role::Rook,
                                    shakmaty::Role::Queen,
                                    shakmaty::Role::King,
                                ],
                            };
                            let colors = match color_opt {
                                Some(c) => vec![c],
                                None => vec![Color::White, Color::Black],
                            };
                            for c in colors {
                                for &r in &roles {
                                    char_pieces.push(Piece { color: c, role: r });
                                }
                            }
                        } else {
                            all_chars_valid = false;
                            break;
                        }
                    }
                    if all_chars_valid && !char_pieces.is_empty() {
                        pieces.extend(char_pieces);
                        continue;
                    }
                }

                if let Some((color_opt, role_opt)) = parse_piece_specifier(&pt) {
                    let roles = match role_opt {
                        Some(r) => vec![r],
                        None => vec![
                            shakmaty::Role::Pawn,
                            shakmaty::Role::Knight,
                            shakmaty::Role::Bishop,
                            shakmaty::Role::Rook,
                            shakmaty::Role::Queen,
                            shakmaty::Role::King,
                        ],
                    };
                    let colors = match color_opt {
                        Some(c) => vec![c],
                        None => vec![Color::White, Color::Black],
                    };
                    for c in colors {
                        for &r in &roles {
                            pieces.push(Piece { color: c, role: r });
                        }
                    }
                } else {
                    return Err(ParseError::new(
                        format!("Invalid piece specifier: {}", pt),
                        pos,
                    ));
                }
            }
            if pieces.is_empty() {
                SquareContent::Occupied
            } else if pieces.len() == 1 {
                SquareContent::Piece(pieces[0])
            } else {
                SquareContent::AnyOf(pieces)
            }
        } else if let Some(Token::Ident(ref id)) = self.peek() {
            let id_clone = id.clone();
            let id_low = id_clone.to_lowercase();
            if id_low == "piece" || id_low == "square" {
                self.advance();
                return self.parse_piece_on_square_with_filter(Some(modifier_squares));
            } else if id_low == "white_pieces" || id_low == "white_piece" {
                self.advance();
                SquareContent::Color(Color::White)
            } else if id_low == "black_pieces" || id_low == "black_piece" {
                self.advance();
                SquareContent::Color(Color::Black)
            } else if id_low == "occupied" || id_low == "pieces" || id_low == "any_piece" {
                self.advance();
                SquareContent::Occupied
            } else if id_low == "empty" {
                self.advance();
                SquareContent::Empty
            } else if let Some((color, role)) = parse_piece_specifier(&id_clone) {
                self.advance();
                match (color, role) {
                    (Some(c), Some(r)) => SquareContent::Piece(Piece { color: c, role: r }),
                    (None, Some(r)) => SquareContent::Role(r),
                    (Some(c), None) => SquareContent::Color(c),
                    (None, None) => SquareContent::Occupied,
                }
            } else if id_low == "squares" {
                self.advance();
                SquareContent::Occupied
            } else {
                SquareContent::Occupied
            }
        } else {
            SquareContent::Occupied
        };

        if self.match_ident("on") || self.match_ident("in") {
            let squares = self.parse_square_set()?;
            let final_squares: Vec<Square> = squares
                .into_iter()
                .filter(|sq| modifier_squares.contains(sq))
                .collect();
            let is_cmp = matches!(
                self.peek(),
                Some(
                    Token::Eq
                        | Token::Neq
                        | Token::Gt
                        | Token::Gte
                        | Token::Lt
                        | Token::Lte
                        | Token::Colon
                )
            );
            if self.match_ident("count") || is_cmp {
                let op = self.parse_comparison_op();
                let count = self.expect_number()? as usize;
                return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                    content,
                    squares: Some(final_squares),
                    op,
                    count,
                }));
            }
            if final_squares.len() == 1 {
                let mut map = HashMap::new();
                map.insert(final_squares[0], content);
                return Ok(SearchQuery::Position(PositionPattern::Squares(map)));
            } else {
                return Ok(SearchQuery::Position(PositionPattern::MultiSquare {
                    content,
                    squares: final_squares,
                }));
            }
        }

        let is_cmp = matches!(
            self.peek(),
            Some(
                Token::Eq
                    | Token::Neq
                    | Token::Gt
                    | Token::Gte
                    | Token::Lt
                    | Token::Lte
                    | Token::Colon
            )
        );

        if self.match_ident("count") || is_cmp {
            let op = self.parse_comparison_op();
            let count = self.expect_number()? as usize;
            return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                content,
                squares: Some(modifier_squares),
                op,
                count,
            }));
        }

        Ok(SearchQuery::Position(PositionPattern::MultiSquare {
            content,
            squares: modifier_squares,
        }))
    }
}
