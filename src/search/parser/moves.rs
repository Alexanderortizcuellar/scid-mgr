use shakmaty::{Color, Piece, Square};
use std::str::FromStr;

use super::helpers::{expand_square_specifier, parse_piece_specifier};
use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{MovePattern, PathPattern, PathStep, SearchQuery, SquareContent};

impl<'a> QueryParser<'a> {
    pub(crate) fn parse_path_expr(&mut self, consecutive: bool) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        let mut steps = Vec::new();
        let mut moves = Vec::new();

        if let Some(Token::StringLit(s)) = self.peek() {
            let s_clone = s.clone();
            self.advance();
            for token in s_clone.split_whitespace() {
                if token.is_empty() || (token.ends_with('.') && !token.starts_with('.')) {
                    continue;
                }
                if token == "..." || token == "--*" || token == "_*" || token == "*" {
                    steps.push(PathStep::Gap { min: 0, max: None });
                } else if token == "--+" || token == "_+" || token == "+" {
                    steps.push(PathStep::Gap { min: 1, max: None });
                } else if let Some(pat) = super::helpers::parse_path_move_token(token) {
                    moves.push(pat.clone());
                    steps.push(PathStep::Move(pat));
                }
            }
            return Ok(SearchQuery::Path(PathPattern {
                steps,
                moves,
                consecutive,
                max_gap_plies: if consecutive { None } else { Some(30) },
                start_ply_range: None,
            }));
        }

        if let Some(Token::LBracket) = self.peek() {
            self.advance();
            while let Some(tok) = self.peek() {
                if let Token::RBracket = tok {
                    self.advance();
                    break;
                }
                if let Token::Comma = tok {
                    self.advance();
                    continue;
                }

                if let Token::Dots = tok {
                    self.advance();
                    steps.push(PathStep::Gap { min: 0, max: None });
                    continue;
                }

                if let Token::StringLit(s) = tok {
                    let s_clone = s.clone();
                    self.advance();
                    if let Some(pat) = super::helpers::parse_path_move_token(&s_clone) {
                        moves.push(pat.clone());
                        steps.push(PathStep::Move(pat));
                    }
                    continue;
                }

                if let Token::LBrace = tok {
                    self.advance();
                    let mut min = 0;
                    let mut max = None;
                    if let Some(Token::Number(n)) = self.peek() {
                        min = *n as usize;
                        self.advance();
                    }
                    if let Some(Token::Comma) = self.peek() {
                        self.advance();
                        if let Some(Token::Number(n2)) = self.peek() {
                            max = Some(*n2 as usize);
                            self.advance();
                        }
                    }
                    if let Some(Token::RBrace) = self.peek() {
                        self.advance();
                    }
                    steps.push(PathStep::Gap { min, max });
                    continue;
                }

                if let Token::Ident(s) = tok {
                    let mut full_token = s.clone();
                    self.advance();

                    if full_token == "--" && matches!(self.peek(), Some(Token::LBrace)) {
                        self.advance();
                        let mut min = 0;
                        let mut max = None;
                        if let Some(Token::Number(n)) = self.peek() {
                            min = *n as usize;
                            self.advance();
                        }
                        if let Some(Token::Comma) = self.peek() {
                            self.advance();
                            if let Some(Token::Number(n2)) = self.peek() {
                                max = Some(*n2 as usize);
                                self.advance();
                            }
                        }
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        steps.push(PathStep::Gap { min, max });
                        continue;
                    }

                    if let Some(Token::Number(n)) = self.peek() {
                        full_token.push_str(&n.to_string());
                        self.advance();
                    }

                    // Check for nested bracket like [x], [q,r,p], [e4,d5]
                    if let Some(Token::LBracket) = self.peek() {
                        self.advance();
                        full_token.push('[');
                        while let Some(b_tok) = self.peek() {
                            if let Token::RBracket = b_tok {
                                self.advance();
                                full_token.push(']');
                                break;
                            }
                            match b_tok {
                                Token::Ident(s) => full_token.push_str(s),
                                Token::Number(n) => full_token.push_str(&n.to_string()),
                                Token::Comma => full_token.push(','),
                                Token::Eq => full_token.push('='),
                                Token::StringLit(s) => full_token.push_str(&format!("\"{}\"", s)),
                                _ => {}
                            }
                            self.advance();
                        }

                        // After bracket, might have target square/ident like 'h7' or 'd8'
                        if let Some(Token::Ident(target_id)) = self.peek() {
                            full_token.push_str(target_id);
                            self.advance();
                            if let Some(Token::Number(target_n)) = self.peek() {
                                full_token.push_str(&target_n.to_string());
                                self.advance();
                            }
                        }
                    }

                    if let Some(Token::Eq) = self.peek() {
                        self.advance();
                        full_token.push('=');
                        if let Some(Token::Ident(promo_piece)) = self.peek() {
                            full_token.push_str(promo_piece);
                            self.advance();
                        } else if let Some(Token::StringLit(promo_str)) = self.peek() {
                            full_token.push_str(&format!("\"{}\"", promo_str));
                            self.advance();
                        }
                    }

                    if let Some(Token::Ident(s2)) = self.peek() {
                        if s2.starts_with('=')
                            || s2.starts_with('#')
                            || s2.starts_with('+')
                            || s2.starts_with('!')
                            || s2.starts_with('?')
                        {
                            full_token.push_str(s2);
                            self.advance();
                        }
                    }

                    if full_token == "..."
                        || full_token == "--*"
                        || full_token == "_*"
                        || full_token == "*"
                    {
                        steps.push(PathStep::Gap { min: 0, max: None });
                        continue;
                    }
                    if full_token == "--+" || full_token == "_+" || full_token == "+" {
                        steps.push(PathStep::Gap { min: 1, max: None });
                        continue;
                    }

                    if let Some(pat) = super::helpers::parse_path_move_token(&full_token) {
                        moves.push(pat.clone());
                        steps.push(PathStep::Move(pat));
                    }
                    continue;
                }

                self.advance();
            }

            return Ok(SearchQuery::Path(PathPattern {
                steps,
                moves,
                consecutive,
                max_gap_plies: if consecutive { None } else { Some(30) },
                start_ply_range: None,
            }));
        }

        Err(ParseError::new(
            "Expected '[' or string after line/path".to_string(),
            pos,
        ))
    }

    /// Parse a single square or piece, a bracketed list of squares/ranges/pieces, a diagonal/ray, or a range expression
    #[allow(clippy::type_complexity)]
    pub(crate) fn parse_square_or_piece_set(
        &mut self,
    ) -> Result<(Option<Vec<Square>>, Option<Vec<SquareContent>>), ParseError> {
        let pos = self.current_pos();

        if let Some(Token::LBracket) = self.peek() {
            self.advance();
            let mut squares = Vec::new();
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
                if let Token::Ident(ref s) = tok {
                    let s_clone = s.clone();
                    self.advance();
                    let s_low = s_clone.to_lowercase();
                    if s_low == "light" || s_low == "light_squares" {
                        squares.extend(shakmaty::Bitboard::LIGHT_SQUARES);
                    } else if s_low == "dark" || s_low == "dark_squares" {
                        squares.extend(shakmaty::Bitboard::DARK_SQUARES);
                    } else if s_low == "white_pieces" || s_low == "white_piece" {
                        pieces.push(SquareContent::Color(Color::White));
                    } else if s_low == "black_pieces" || s_low == "black_piece" {
                        pieces.push(SquareContent::Color(Color::Black));
                    } else if s_low == "occupied" || s_low == "pieces" || s_low == "any_piece" {
                        pieces.push(SquareContent::Occupied);
                    } else if s_low == "empty" {
                        pieces.push(SquareContent::Empty);
                    } else if let Ok(sq) = Square::from_str(&s_low) {
                        squares.push(sq);
                    } else if let Some(expanded) = expand_square_specifier(&s_clone) {
                        squares.extend(expanded);
                    } else if let Some((color, role)) = parse_piece_specifier(&s_clone) {
                        match (color, role) {
                            (Some(c), Some(r)) => {
                                pieces.push(SquareContent::Piece(Piece { color: c, role: r }))
                            }
                            (Some(c), None) => pieces.push(SquareContent::Color(c)),
                            (None, Some(r)) => {
                                pieces.push(SquareContent::Piece(Piece {
                                    color: Color::White,
                                    role: r,
                                }));
                                pieces.push(SquareContent::Piece(Piece {
                                    color: Color::Black,
                                    role: r,
                                }));
                            }
                            (None, None) => pieces.push(SquareContent::Occupied),
                        }
                    } else {
                        return Err(ParseError::new(
                            format!("Unrecognized square or piece specifier: {}", s_clone),
                            pos,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        format!("Expected square or piece identifier, found {:?}", tok),
                        pos,
                    ));
                }
            }

            let sq_opt = if squares.is_empty() {
                None
            } else {
                Some(squares)
            };
            let pc_opt = if pieces.is_empty() {
                None
            } else {
                Some(pieces)
            };
            return Ok((sq_opt, pc_opt));
        }

        if let Some(Token::Ident(ref s)) = self.peek() {
            let s_clone = s.clone();
            let s_low = s_clone.to_lowercase();
            if s_low == "diag" || s_low == "diagonal" || s_low == "ray" {
                let sqs = self.parse_square_set()?;
                return Ok((Some(sqs), None));
            }
            if s_low == "light" || s_low == "light_squares" {
                self.advance();
                return Ok((
                    Some(shakmaty::Bitboard::LIGHT_SQUARES.into_iter().collect()),
                    None,
                ));
            }
            if s_low == "dark" || s_low == "dark_squares" {
                self.advance();
                return Ok((
                    Some(shakmaty::Bitboard::DARK_SQUARES.into_iter().collect()),
                    None,
                ));
            }
            if s_low == "white_pieces" || s_low == "white_piece" {
                self.advance();
                return Ok((None, Some(vec![SquareContent::Color(Color::White)])));
            }
            if s_low == "black_pieces" || s_low == "black_piece" {
                self.advance();
                return Ok((None, Some(vec![SquareContent::Color(Color::Black)])));
            }
            if s_low == "occupied" || s_low == "pieces" || s_low == "any_piece" {
                self.advance();
                return Ok((None, Some(vec![SquareContent::Occupied])));
            }
            if s_low == "empty" {
                self.advance();
                return Ok((None, Some(vec![SquareContent::Empty])));
            }
            if let Ok(sq) = Square::from_str(&s_low) {
                self.advance();
                return Ok((Some(vec![sq]), None));
            }
            if let Some(expanded) = expand_square_specifier(&s_clone) {
                self.advance();
                return Ok((Some(expanded), None));
            }
            if let Some((color, role)) = parse_piece_specifier(&s_clone) {
                self.advance();
                let pc = match (color, role) {
                    (Some(c), Some(r)) => vec![SquareContent::Piece(Piece { color: c, role: r })],
                    (Some(c), None) => vec![SquareContent::Color(c)],
                    (None, Some(r)) => vec![
                        SquareContent::Piece(Piece {
                            color: Color::White,
                            role: r,
                        }),
                        SquareContent::Piece(Piece {
                            color: Color::Black,
                            role: r,
                        }),
                    ],
                    (None, None) => vec![SquareContent::Occupied],
                };
                return Ok((None, Some(pc)));
            }
        }

        Err(ParseError::new(
            "Expected square or piece specifier".to_string(),
            pos,
        ))
    }

    pub(crate) fn parse_move_filter(
        &mut self,
        is_legal_init: bool,
    ) -> Result<SearchQuery, ParseError> {
        let mut pattern = MovePattern {
            is_legal: is_legal_init,
            ..Default::default()
        };

        if let Some(Token::StringLit(s)) = self.peek() {
            let san = s.clone();
            self.advance();
            pattern.san = Some(san);
            return Ok(SearchQuery::Move(pattern));
        }

        while let Some(tok) = self.peek() {
            match tok {
                Token::Ident(ref id) => {
                    let id_low = id.to_lowercase();
                    match id_low.as_str() {
                        "legal" => {
                            self.advance();
                            pattern.is_legal = true;
                        }
                        "from" => {
                            self.advance();
                            let (sqs_opt, pcs_opt) = self.parse_square_or_piece_set()?;
                            if let Some(ref sqs) = sqs_opt {
                                if sqs.len() == 1 {
                                    pattern.from = Some(sqs[0]);
                                }
                            }
                            pattern.from_squares = sqs_opt;
                            pattern.from_pieces = pcs_opt;
                        }
                        "to" => {
                            self.advance();
                            let (sqs_opt, pcs_opt) = self.parse_square_or_piece_set()?;
                            if let Some(ref sqs) = sqs_opt {
                                if sqs.len() == 1 {
                                    pattern.to = Some(sqs[0]);
                                }
                            }
                            pattern.to_squares = sqs_opt;
                            pattern.to_pieces = pcs_opt;
                        }
                        "piece" => {
                            self.advance();
                            let piece_ident = self.expect_ident()?;
                            if let Some((color, role)) = parse_piece_specifier(&piece_ident) {
                                pattern.color = color;
                                pattern.role = role;
                            }
                        }
                        "capture" | "is_capture" => {
                            self.advance();
                            pattern.is_capture = Some(true);
                        }
                        "promote" | "promotion" | "promotes" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                                || matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            {
                                self.advance();
                            }

                            if let Some(Token::LBracket) = self.peek() {
                                self.advance();
                                let mut roles = Vec::new();
                                while let Some(inner_tok) = self.peek() {
                                    if let Token::RBracket = inner_tok {
                                        self.advance();
                                        break;
                                    }
                                    if let Token::Comma = inner_tok {
                                        self.advance();
                                        continue;
                                    }
                                    if let Token::Ident(ref s) = inner_tok {
                                        let s_clone = s.clone();
                                        self.advance();
                                        for ch in s_clone.chars() {
                                            match ch.to_ascii_uppercase() {
                                                'Q' => roles.push(shakmaty::Role::Queen),
                                                'R' => roles.push(shakmaty::Role::Rook),
                                                'B' => roles.push(shakmaty::Role::Bishop),
                                                'N' => roles.push(shakmaty::Role::Knight),
                                                _ => {}
                                            }
                                        }
                                    } else {
                                        self.advance();
                                    }
                                }
                                if roles.len() == 1 {
                                    pattern.promotion = Some(roles[0]);
                                } else if !roles.is_empty() {
                                    pattern.promotions = Some(roles);
                                }
                            } else if let Some(Token::StringLit(ref s)) = self.peek() {
                                let s_clone = s.clone();
                                self.advance();
                                let mut roles = Vec::new();
                                for ch in s_clone.chars() {
                                    match ch.to_ascii_uppercase() {
                                        'Q' => roles.push(shakmaty::Role::Queen),
                                        'R' => roles.push(shakmaty::Role::Rook),
                                        'B' => roles.push(shakmaty::Role::Bishop),
                                        'N' => roles.push(shakmaty::Role::Knight),
                                        _ => {}
                                    }
                                }
                                if roles.len() == 1 {
                                    pattern.promotion = Some(roles[0]);
                                } else if !roles.is_empty() {
                                    pattern.promotions = Some(roles);
                                }
                            } else if let Some(Token::Ident(ref s)) = self.peek() {
                                let s_low = s.to_lowercase();
                                if matches!(
                                    s_low.as_str(),
                                    "count"
                                        | "check"
                                        | "is_check"
                                        | "from"
                                        | "to"
                                        | "piece"
                                        | "capture"
                                        | "is_capture"
                                        | "legal"
                                ) {
                                    // bare promote -> any promotion
                                    pattern.promotions = Some(vec![
                                        shakmaty::Role::Queen,
                                        shakmaty::Role::Rook,
                                        shakmaty::Role::Bishop,
                                        shakmaty::Role::Knight,
                                    ]);
                                } else {
                                    let s_clone = s.clone();
                                    self.advance();
                                    let mut roles = Vec::new();
                                    match s_low.as_str() {
                                        "queen" => roles.push(shakmaty::Role::Queen),
                                        "rook" => roles.push(shakmaty::Role::Rook),
                                        "bishop" => roles.push(shakmaty::Role::Bishop),
                                        "knight" => roles.push(shakmaty::Role::Knight),
                                        _ => {
                                            for ch in s_clone.chars() {
                                                match ch.to_ascii_uppercase() {
                                                    'Q' => roles.push(shakmaty::Role::Queen),
                                                    'R' => roles.push(shakmaty::Role::Rook),
                                                    'B' => roles.push(shakmaty::Role::Bishop),
                                                    'N' => roles.push(shakmaty::Role::Knight),
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                    if roles.len() == 1 {
                                        pattern.promotion = Some(roles[0]);
                                    } else if !roles.is_empty() {
                                        pattern.promotions = Some(roles);
                                    } else {
                                        pattern.promotions = Some(vec![
                                            shakmaty::Role::Queen,
                                            shakmaty::Role::Rook,
                                            shakmaty::Role::Bishop,
                                            shakmaty::Role::Knight,
                                        ]);
                                    }
                                }
                            } else {
                                pattern.promotions = Some(vec![
                                    shakmaty::Role::Queen,
                                    shakmaty::Role::Rook,
                                    shakmaty::Role::Bishop,
                                    shakmaty::Role::Knight,
                                ]);
                            }
                        }
                        "check" | "is_check" => {
                            self.advance();
                            pattern.is_check = Some(true);
                        }
                        "count" => {
                            self.advance();
                            let op = self.parse_comparison_op();
                            let count = self.expect_number()? as usize;
                            pattern.count_predicate = Some((op, count));
                            break;
                        }
                        _ => {
                            if let Some((color, role)) = parse_piece_specifier(id) {
                                self.advance();
                                pattern.color = color;
                                pattern.role = role;
                            } else {
                                break;
                            }
                        }
                    }
                }
                Token::Eq
                | Token::Neq
                | Token::Gt
                | Token::Gte
                | Token::Lt
                | Token::Lte
                | Token::Colon => {
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    pattern.count_predicate = Some((op, count));
                    break;
                }
                _ => break,
            }
        }

        Ok(SearchQuery::Move(pattern))
    }
}
