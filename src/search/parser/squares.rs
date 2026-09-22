#![allow(
    clippy::if_same_then_else,
    clippy::needless_bool,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use shakmaty::{Bitboard, Color, Piece, Square};
use std::collections::HashMap;
use std::str::FromStr;

use super::helpers::{expand_diagonal_ray, expand_square_specifier, parse_piece_specifier};
use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{
    PositionPattern, SearchQuery, SetPredicate, SquareContent, SquareSetExpr,
};

impl<'a> QueryParser<'a> {
    /// Parse a single square, a bracketed list of squares/ranges, a diagonal/ray, or a direction expression
    pub(crate) fn parse_square_set(&mut self) -> Result<Vec<Square>, ParseError> {
        let pos = self.current_pos();

        // Check for diag / diagonal / ray keyword prefix: diag[a1-h8] or diag(a1, h8) or ray(up, d4)
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_lower = id.to_lowercase();
            if id_lower == "ray"
                || ((id_lower == "diag" || id_lower == "diagonal")
                    && matches!(
                        self.peek_nth(1),
                        Some(Token::LParen) | Some(Token::LBracket)
                    ))
            {
                self.advance();
                return self.parse_diagonal_or_ray_expression();
            }
            if let Some(dir) = super::helpers::parse_direction_ident(id) {
                self.advance();
                return self.parse_direction_expression(dir);
            }
        }

        // Bracketed list: [sq1, a1-h2, a1..h8, up 1 d4, ...]
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
                    if id_lower == "ray"
                        || ((id_lower == "diag" || id_lower == "diagonal")
                            && matches!(
                                self.peek_nth(1),
                                Some(Token::LParen) | Some(Token::LBracket)
                            ))
                    {
                        self.advance();
                        let diag_sqs = self.parse_diagonal_or_ray_expression()?;
                        squares.extend(diag_sqs);
                        continue;
                    }
                    if let Some(dir) = super::helpers::parse_direction_ident(id) {
                        self.advance();
                        let dir_sqs = self.parse_direction_expression(dir)?;
                        squares.extend(dir_sqs);
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
            squares.sort_unstable();
            squares.dedup();
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

    /// Parse a direction expression (e.g. up 1 d4, up 1 3 d4, up d4, orthogonal d4, diagonal 1 [d4, e5])
    pub(crate) fn parse_direction_expression(
        &mut self,
        dir: crate::search::query::Direction,
    ) -> Result<Vec<Square>, ParseError> {
        let is_paren = matches!(self.peek(), Some(Token::LParen));
        if is_paren {
            self.advance();
        }

        if let Some(Token::Comma) = self.peek() {
            self.advance();
        }

        let mut min_dist = 1;
        let mut max_dist = 7;

        if let Some(Token::Number(n1)) = self.peek() {
            let val1 = *n1;
            self.advance();
            if let Some(Token::Comma) = self.peek() {
                self.advance();
            }
            if let Some(Token::Number(n2)) = self.peek() {
                let val2 = *n2;
                self.advance();
                min_dist = val1.min(val2).max(1) as usize;
                max_dist = val1.max(val2).clamp(1, 7) as usize;
            } else if let Some(Token::DotDot) = self.peek() {
                self.advance();
                if let Some(Token::Number(n2)) = self.peek() {
                    let val2 = *n2;
                    self.advance();
                    min_dist = val1.min(val2).max(1) as usize;
                    max_dist = val1.max(val2).clamp(1, 7) as usize;
                } else {
                    min_dist = val1.max(1) as usize;
                    max_dist = 7;
                }
            } else {
                min_dist = val1.unsigned_abs().max(1) as usize;
                max_dist = min_dist;
            }
        }

        if let Some(Token::Comma) = self.peek() {
            self.advance();
        }

        let base_squares = self.parse_square_set()?;

        if is_paren {
            if let Some(Token::RParen) = self.peek() {
                self.advance();
            }
        }

        Ok(dir.expand_squares(&base_squares, min_dist, max_dist))
    }

    /// Parse a diagonal or ray expression (e.g. diag(a1, h8), ray(up, d4), diag[a1-h8], diag a1-h8)
    pub(crate) fn parse_diagonal_or_ray_expression(&mut self) -> Result<Vec<Square>, ParseError> {
        let pos = self.current_pos();
        let is_bracket = matches!(self.peek(), Some(Token::LBracket));
        let is_paren = matches!(self.peek(), Some(Token::LParen));

        if is_bracket || is_paren {
            self.advance();
            if let Some(Token::Ident(ref first_id)) = self.peek() {
                if let Some(dir) = super::helpers::parse_direction_ident(first_id) {
                    self.advance();
                    let res = self.parse_direction_expression(dir)?;
                    if is_bracket {
                        self.expect_token(Token::RBracket)?;
                    } else {
                        self.expect_token(Token::RParen)?;
                    }
                    return Ok(res);
                }
            }
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
            if let Some(Token::Ident(ref first_id)) = self.peek() {
                if let Some(dir) = super::helpers::parse_direction_ident(first_id) {
                    self.advance();
                    return self.parse_direction_expression(dir);
                }
            }
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

    /// Entry point for parsing a square set expression query (predicate or comparison)
    pub(crate) fn parse_square_set_query(&mut self) -> Result<SearchQuery, ParseError> {
        let left_expr = self.parse_square_set_expr()?;

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
            if let Some(Token::Number(n)) = self.peek() {
                let count = *n as usize;
                self.advance();
                return Ok(SearchQuery::SquareSet(SetPredicate::CountComparison {
                    expr: left_expr,
                    op,
                    count,
                }));
            } else if self.is_square_set_atom_start() {
                let right_expr = self.parse_square_set_expr()?;
                return Ok(SearchQuery::SquareSet(SetPredicate::SetComparison {
                    left: left_expr,
                    op,
                    right: right_expr,
                }));
            }
        }

        Ok(SearchQuery::SquareSet(SetPredicate::NonEmpty(left_expr)))
    }

    /// Parse full square set expression with Pratt precedence: Union (|) -> Difference (\, -) -> Intersection (&, juxtaposition) -> Unary (~) -> Atom
    pub(crate) fn parse_square_set_expr(&mut self) -> Result<SquareSetExpr, ParseError> {
        self.parse_square_set_union()
    }

    pub(crate) fn parse_square_set_union(&mut self) -> Result<SquareSetExpr, ParseError> {
        let mut left = self.parse_square_set_diff()?;

        while let Some(Token::Pipe) = self.peek() {
            self.advance();
            let right = self.parse_square_set_diff()?;
            left = SquareSetExpr::Union(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    pub(crate) fn parse_square_set_diff(&mut self) -> Result<SquareSetExpr, ParseError> {
        let mut left = self.parse_square_set_intersection()?;

        while let Some(tok) = self.peek() {
            if matches!(tok, Token::Backslash) || matches!(tok, Token::Ident(s) if s == "-") {
                self.advance();
                let right = self.parse_square_set_intersection()?;
                left = SquareSetExpr::Difference(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_square_set_intersection(&mut self) -> Result<SquareSetExpr, ParseError> {
        let mut left = self.parse_square_set_unary()?;

        loop {
            if let Some(Token::Ampersand) = self.peek() {
                self.advance();
                let right = self.parse_square_set_unary()?;
                left = SquareSetExpr::Intersection(Box::new(left), Box::new(right));
            } else if self.match_ident("on") || self.match_ident("in") {
                let right = self.parse_square_set_unary()?;
                left = SquareSetExpr::Intersection(Box::new(left), Box::new(right));
            } else if self.is_juxtaposition_square_start() {
                let right = self.parse_square_set_unary()?;
                left = SquareSetExpr::Intersection(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn is_juxtaposition_square_start(&self) -> bool {
        match self.peek() {
            Some(Token::LBracket) => !self.is_bracket_piece_list(),
            Some(Token::Ident(ref id)) => {
                let id_low = id.to_lowercase();
                if id_low == "light"
                    || id_low == "dark"
                    || id_low == "light_squares"
                    || id_low == "dark_squares"
                    || id_low == "all_squares"
                {
                    return true;
                }
                expand_square_specifier(id).is_some()
            }
            _ => false,
        }
    }

    pub(crate) fn parse_square_set_unary(&mut self) -> Result<SquareSetExpr, ParseError> {
        if let Some(Token::Tilde) = self.peek() {
            self.advance();
            let inner = self.parse_square_set_unary()?;
            return Ok(SquareSetExpr::Complement(Box::new(inner)));
        }
        if let Some(Token::Ident(ref s)) = self.peek() {
            if s == "!" {
                self.advance();
                let inner = self.parse_square_set_unary()?;
                return Ok(SquareSetExpr::Complement(Box::new(inner)));
            }
            if let Some(dir) = super::helpers::parse_direction_ident(s) {
                self.advance();
                let is_paren = matches!(self.peek(), Some(Token::LParen));
                if is_paren {
                    self.advance();
                }
                if let Some(Token::Comma) = self.peek() {
                    self.advance();
                }
                let mut min_dist = 1;
                let mut max_dist = 1;
                let mut has_dist = false;

                if let Some(Token::Number(n1)) = self.peek() {
                    has_dist = true;
                    let val1 = *n1;
                    self.advance();
                    if let Some(Token::Comma) = self.peek() {
                        self.advance();
                    }
                    if let Some(Token::Number(n2)) = self.peek() {
                        let val2 = *n2;
                        self.advance();
                        min_dist = val1.min(val2).max(1) as usize;
                        max_dist = val1.max(val2).clamp(1, 7) as usize;
                    } else if let Some(Token::DotDot) = self.peek() {
                        self.advance();
                        if let Some(Token::Number(n2)) = self.peek() {
                            let val2 = *n2;
                            self.advance();
                            min_dist = val1.min(val2).max(1) as usize;
                            max_dist = val1.max(val2).clamp(1, 7) as usize;
                        } else {
                            min_dist = val1.max(1) as usize;
                            max_dist = 7;
                        }
                    } else {
                        min_dist = val1.unsigned_abs().max(1) as usize;
                        max_dist = min_dist;
                    }
                }

                if !has_dist {
                    min_dist = 1;
                    max_dist = 7;
                }

                if let Some(Token::Comma) = self.peek() {
                    self.advance();
                }

                let inner = self.parse_square_set_unary()?;

                if is_paren {
                    if let Some(Token::RParen) = self.peek() {
                        self.advance();
                    }
                }

                return Ok(SquareSetExpr::Shift {
                    direction: dir,
                    min_dist,
                    max_dist,
                    expr: Box::new(inner),
                });
            }
        }

        self.parse_square_set_atom()
    }

    pub(crate) fn is_square_set_atom_start(&self) -> bool {
        match self.peek() {
            Some(Token::LParen) | Some(Token::LBracket) | Some(Token::Tilde) => true,
            Some(Token::Ident(ref id)) => {
                let id_low = id.to_lowercase();
                if id_low == "and"
                    || id_low == "or"
                    || id_low == "not"
                    || id_low == "cql"
                    || id_low == "count"
                    || id == "-"
                {
                    return false;
                }
                if id.starts_with('$') {
                    return true;
                }
                if id_low == "attacks"
                    || id_low == "attackers"
                    || id_low == "ray"
                    || id_low == "diag"
                    || id_low == "diagonal"
                    || id_low == "between"
                    || id_low == "light"
                    || id_low == "dark"
                    || id_low == "all"
                    || id_low == "all_squares"
                    || id_low == "occupied"
                    || id_low == "empty"
                    || id == "_"
                    || id == "."
                    || id_low == "white_pieces"
                    || id_low == "black_pieces"
                    || id_low == "pieces"
                {
                    return true;
                }
                if super::helpers::parse_direction_ident(id).is_some() {
                    return true;
                }
                if parse_piece_specifier(id).is_some() {
                    return true;
                }
                if expand_square_specifier(id).is_some() {
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    pub(crate) fn parse_square_set_atom(&mut self) -> Result<SquareSetExpr, ParseError> {
        let pos = self.current_pos();

        // 1. Parenthesized sub-expression: `( A | B )`
        if let Some(Token::LParen) = self.peek() {
            self.advance();
            let expr = self.parse_square_set_expr()?;
            self.expect_token(Token::RParen)?;
            return Ok(expr);
        }

        // 2. Bracketed list: can be empty set `[]`, pieces `[N, B]`, `[Q, R]`, `[qr]`, `[q]`, or squares `[c1, f1]`, `[a1..h8]`
        if let Some(Token::LBracket) = self.peek() {
            if let Some(Token::RBracket) = self.peek_nth(1) {
                self.advance(); // consume '['
                self.advance(); // consume ']'
                return Ok(SquareSetExpr::Squares(Bitboard::EMPTY));
            }
            if self.is_bracket_piece_list() {
                self.advance(); // consume '['
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
                    // Multi-char piece string e.g. "qr", "RBN", "Aa"
                    if pt.len() > 1 && parse_piece_specifier(&pt).is_none() {
                        let mut all_valid = true;
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
                                all_valid = false;
                                break;
                            }
                        }
                        if all_valid && !char_pieces.is_empty() {
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
                            format!("Invalid piece specifier in piece set: {}", pt),
                            pos,
                        ));
                    }
                }

                let content = if pieces.is_empty() {
                    SquareContent::Occupied
                } else if pieces.len() == 1 {
                    SquareContent::Piece(pieces[0])
                } else {
                    SquareContent::AnyOf(pieces)
                };
                return Ok(SquareSetExpr::Piece(content));
            }

            let squares = self.parse_square_set()?;
            let mut bb = Bitboard::EMPTY;
            for sq in squares {
                bb.add(sq);
            }
            return Ok(SquareSetExpr::Squares(bb));
        }

        // 3. Variable reference: `$target`, `$attacker`
        if let Some(Token::Ident(ref id)) = self.peek() {
            if id.starts_with('$') {
                let var_name = id.clone();
                self.advance();
                return Ok(SquareSetExpr::Variable(var_name));
            }
        }

        // 4. Function calls or keywords
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            match id_low.as_str() {
                "attacks" => {
                    self.advance();
                    self.expect_token(Token::LParen)?;
                    let attacker = self.parse_square_set_expr()?;
                    self.expect_token(Token::Comma)?;
                    let target = self.parse_square_set_expr()?;
                    self.expect_token(Token::RParen)?;
                    return Ok(SquareSetExpr::Attacks {
                        attacker: Box::new(attacker),
                        target: Box::new(target),
                    });
                }
                "attackers" => {
                    self.advance();
                    self.expect_token(Token::LParen)?;
                    let attacker = self.parse_square_set_expr()?;
                    self.expect_token(Token::Comma)?;
                    let target = self.parse_square_set_expr()?;
                    self.expect_token(Token::RParen)?;
                    return Ok(SquareSetExpr::Attackers {
                        attacker: Box::new(attacker),
                        target: Box::new(target),
                    });
                }
                "between" => {
                    self.advance();
                    self.expect_token(Token::LParen)?;
                    let from = self.parse_square_set_expr()?;
                    self.expect_token(Token::Comma)?;
                    let to = self.parse_square_set_expr()?;
                    self.expect_token(Token::RParen)?;
                    return Ok(SquareSetExpr::Between {
                        from: Box::new(from),
                        to: Box::new(to),
                    });
                }
                "ray" | "diag" | "diagonal" => {
                    if matches!(
                        self.peek_nth(1),
                        Some(Token::LParen) | Some(Token::LBracket)
                    ) {
                        self.advance();
                        let sqs = self.parse_diagonal_or_ray_expression()?;
                        let mut bb = Bitboard::EMPTY;
                        for sq in sqs {
                            bb.add(sq);
                        }
                        return Ok(SquareSetExpr::Squares(bb));
                    }
                }
                "light" | "light_squares" => {
                    self.advance();
                    return Ok(SquareSetExpr::Squares(Bitboard::LIGHT_SQUARES));
                }
                "dark" | "dark_squares" => {
                    self.advance();
                    return Ok(SquareSetExpr::Squares(Bitboard::DARK_SQUARES));
                }
                "all" | "all_squares" | "board" => {
                    self.advance();
                    return Ok(SquareSetExpr::Squares(!Bitboard::EMPTY));
                }
                "occupied" | "pieces" | "any_piece" => {
                    self.advance();
                    return Ok(SquareSetExpr::Piece(SquareContent::Occupied));
                }
                "empty" | "_" | "." => {
                    self.advance();
                    return Ok(SquareSetExpr::Piece(SquareContent::Empty));
                }
                "white_pieces" | "white_piece" => {
                    self.advance();
                    return Ok(SquareSetExpr::Piece(SquareContent::Color(Color::White)));
                }
                "black_pieces" | "black_piece" => {
                    self.advance();
                    return Ok(SquareSetExpr::Piece(SquareContent::Color(Color::Black)));
                }
                _ => {}
            }

            if let Some(dir) = super::helpers::parse_direction_ident(id) {
                self.advance();
                let sqs = self.parse_direction_expression(dir)?;
                let mut bb = Bitboard::EMPTY;
                for sq in sqs {
                    bb.add(sq);
                }
                return Ok(SquareSetExpr::Squares(bb));
            }

            if let Some((color_opt, role_opt)) = parse_piece_specifier(id) {
                self.advance();
                let content = match (color_opt, role_opt) {
                    (Some(c), Some(r)) => SquareContent::Piece(Piece { color: c, role: r }),
                    (None, Some(r)) => SquareContent::Role(r),
                    (Some(c), None) => SquareContent::Color(c),
                    (None, None) => SquareContent::Occupied,
                };
                return Ok(SquareSetExpr::Piece(content));
            }

            if let Some(sqs) = expand_square_specifier(id) {
                self.advance();
                let mut bb = Bitboard::EMPTY;
                for sq in sqs {
                    bb.add(sq);
                }
                return Ok(SquareSetExpr::Squares(bb));
            }
        }

        Err(ParseError::new(
            format!("Expected square set expression atom at pos {}", pos),
            pos,
        ))
    }

    fn is_bracket_piece_list(&self) -> bool {
        let mut idx = 1;
        let mut has_items = false;
        while let Some((_, tok)) = self.tokens.get(self.pos + idx) {
            match tok {
                Token::RBracket => return has_items,
                Token::Comma => {
                    idx += 1;
                    continue;
                }
                Token::Ident(ref s) => {
                    has_items = true;
                    if parse_piece_specifier(s).is_some() {
                        idx += 1;
                        continue;
                    }
                    if s.len() > 1
                        && s.chars()
                            .all(|ch| parse_piece_specifier(&ch.to_string()).is_some())
                    {
                        idx += 1;
                        continue;
                    }
                    if expand_square_specifier(s).is_some() {
                        return false;
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        false
    }
}
