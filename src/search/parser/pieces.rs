use shakmaty::{Bitboard, Color, Piece, Square};
use std::collections::HashMap;
use std::str::FromStr;

use super::helpers::parse_piece_specifier;
use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{
    PawnPredicate, PieceMatcher, PositionPattern, SearchQuery, SetPredicate, SquareContent,
    SquareSetExpr, VariableDomain,
};

impl<'a> QueryParser<'a> {
    pub(crate) fn parse_piece_on_square(&mut self) -> Result<SearchQuery, ParseError> {
        self.parse_piece_on_square_with_filter(None)
    }

    pub(crate) fn parse_piece_on_square_with_filter(
        &mut self,
        square_filter: Option<Vec<Square>>,
    ) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();

        // Check if next token is "light" or "dark" modifier
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            if id_low == "light" || id_low == "light_squares" {
                self.advance();
                return self.parse_color_modified_piece(true);
            } else if id_low == "dark" || id_low == "dark_squares" {
                self.advance();
                return self.parse_color_modified_piece(false);
            }
        }

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
                // If pt is a multi-letter string like "Aa", "Qq", "RBN", "rnb", "kq"
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
        } else {
            let piece_token = self.expect_ident()?;
            let (color, role) = parse_piece_specifier(&piece_token).ok_or_else(|| {
                ParseError::new(format!("Invalid piece specifier: {}", piece_token), pos)
            })?;

            match (color, role) {
                (Some(c), Some(r)) => SquareContent::Piece(Piece { color: c, role: r }),
                (None, Some(r)) => SquareContent::Role(r),
                (Some(c), None) => SquareContent::Color(c),
                (None, None) => SquareContent::Occupied,
            }
        };

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
                return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                    content,
                    squares: square_filter,
                    op,
                    count,
                }));
            } else if self.is_square_set_atom_start() {
                let right_expr = self.parse_square_set_expr()?;
                let mut left_expr = SquareSetExpr::Piece(content);
                if let Some(filter_sqs) = square_filter {
                    let mut bb = Bitboard::EMPTY;
                    for sq in filter_sqs {
                        bb.add(sq);
                    }
                    left_expr = SquareSetExpr::Intersection(
                        Box::new(left_expr),
                        Box::new(SquareSetExpr::Squares(bb)),
                    );
                }
                return Ok(SearchQuery::SquareSet(SetPredicate::SetComparison {
                    left: left_expr,
                    op,
                    right: right_expr,
                }));
            } else {
                let count = self.expect_number()? as usize;
                return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                    content,
                    squares: square_filter,
                    op,
                    count,
                }));
            }
        }

        self.match_ident("on");
        self.match_ident("in");

        let squares = self.parse_square_set()?;
        let final_squares: Vec<Square> = match square_filter {
            Some(filter) => squares
                .into_iter()
                .filter(|sq| filter.contains(sq))
                .collect(),
            None => squares,
        };

        let is_cmp_after_sq = matches!(
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
        if self.match_ident("count") || is_cmp_after_sq {
            let op = self.parse_comparison_op();
            if let Some(Token::Number(n)) = self.peek() {
                let count = *n as usize;
                self.advance();
                return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                    content,
                    squares: Some(final_squares),
                    op,
                    count,
                }));
            } else if self.is_square_set_atom_start() {
                let right_expr = self.parse_square_set_expr()?;
                let mut bb = Bitboard::EMPTY;
                for sq in final_squares {
                    bb.add(sq);
                }
                let left_expr = SquareSetExpr::Intersection(
                    Box::new(SquareSetExpr::Piece(content)),
                    Box::new(SquareSetExpr::Squares(bb)),
                );
                return Ok(SearchQuery::SquareSet(SetPredicate::SetComparison {
                    left: left_expr,
                    op,
                    right: right_expr,
                }));
            } else {
                let count = self.expect_number()? as usize;
                return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                    content,
                    squares: Some(final_squares),
                    op,
                    count,
                }));
            }
        }

        if final_squares.len() == 1 {
            let mut map = HashMap::new();
            map.insert(final_squares[0], content);
            Ok(SearchQuery::Position(PositionPattern::Squares(map)))
        } else {
            Ok(SearchQuery::Position(PositionPattern::MultiSquare {
                content,
                squares: final_squares,
            }))
        }
    }

    pub(crate) fn parse_piece_count_or_squares(
        &mut self,
        content: SquareContent,
    ) -> Result<SearchQuery, ParseError> {
        if self.match_ident("on") || self.match_ident("in") {
            let squares = self.parse_square_set()?;
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
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content,
                        squares: Some(squares),
                        op,
                        count,
                    }));
                } else if self.is_square_set_atom_start() {
                    let right_expr = self.parse_square_set_expr()?;
                    let mut bb = Bitboard::EMPTY;
                    for sq in squares {
                        bb.add(sq);
                    }
                    let left_expr = SquareSetExpr::Intersection(
                        Box::new(SquareSetExpr::Piece(content)),
                        Box::new(SquareSetExpr::Squares(bb)),
                    );
                    return Ok(SearchQuery::SquareSet(SetPredicate::SetComparison {
                        left: left_expr,
                        op,
                        right: right_expr,
                    }));
                } else {
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content,
                        squares: Some(squares),
                        op,
                        count,
                    }));
                }
            }
            if squares.len() == 1 {
                let mut map = HashMap::new();
                map.insert(squares[0], content);
                return Ok(SearchQuery::Position(PositionPattern::Squares(map)));
            } else {
                return Ok(SearchQuery::Position(PositionPattern::MultiSquare {
                    content,
                    squares,
                }));
            }
        }

        self.match_ident("count");
        let op = self.parse_comparison_op();
        if let Some(Token::Number(n)) = self.peek() {
            let count = *n as usize;
            self.advance();
            Ok(SearchQuery::Position(PositionPattern::PieceCount {
                content,
                squares: None,
                op,
                count,
            }))
        } else if self.is_square_set_atom_start() {
            let right_expr = self.parse_square_set_expr()?;
            Ok(SearchQuery::SquareSet(SetPredicate::SetComparison {
                left: SquareSetExpr::Piece(content),
                op,
                right: right_expr,
            }))
        } else {
            let count = self.expect_number()? as usize;
            Ok(SearchQuery::Position(PositionPattern::PieceCount {
                content,
                squares: None,
                op,
                count,
            }))
        }
    }

    pub(crate) fn parse_pawn_color_opt(&mut self) -> Color {
        if let Some(Token::LBracket) = self.peek() {
            self.advance();
            let color = self.parse_color_token();
            if let Some(Token::RBracket) = self.peek() {
                self.advance();
            }
            return color;
        }
        self.parse_color_token()
    }

    pub(crate) fn parse_color_token(&mut self) -> Color {
        if let Some(Token::Ident(s)) = self.peek() {
            let lower = s.to_lowercase();
            if lower == "white" || lower == "w" {
                self.advance();
                return Color::White;
            }
            if lower == "black" || lower == "b" {
                self.advance();
                return Color::Black;
            }
        }
        Color::White
    }

    pub(crate) fn parse_pawn_pred_passed(&mut self) -> Result<SearchQuery, ParseError> {
        let color = self.parse_pawn_color_opt();
        let op = self.parse_comparison_op();
        let count = self.expect_number()? as usize;
        Ok(SearchQuery::Pawn(PawnPredicate::PassedPawns {
            color,
            op,
            count,
        }))
    }

    pub(crate) fn parse_pawn_pred_isolated(&mut self) -> Result<SearchQuery, ParseError> {
        let color = self.parse_pawn_color_opt();
        let op = self.parse_comparison_op();
        let count = self.expect_number()? as usize;
        Ok(SearchQuery::Pawn(PawnPredicate::IsolatedPawns {
            color,
            op,
            count,
        }))
    }

    pub(crate) fn parse_pawn_pred_doubled(&mut self) -> Result<SearchQuery, ParseError> {
        let color = self.parse_pawn_color_opt();
        let op = self.parse_comparison_op();
        let count = self.expect_number()? as usize;
        Ok(SearchQuery::Pawn(PawnPredicate::DoubledPawns {
            color,
            op,
            count,
        }))
    }

    pub(crate) fn parse_pawn_pred_backward(&mut self) -> Result<SearchQuery, ParseError> {
        let color = self.parse_pawn_color_opt();
        let op = self.parse_comparison_op();
        let count = self.expect_number()? as usize;
        Ok(SearchQuery::Pawn(PawnPredicate::BackwardPawns {
            color,
            op,
            count,
        }))
    }

    pub(crate) fn parse_pawn_pred_islands(&mut self) -> Result<SearchQuery, ParseError> {
        let color = self.parse_pawn_color_opt();
        let op = self.parse_comparison_op();
        let count = self.expect_number()? as usize;
        Ok(SearchQuery::Pawn(PawnPredicate::PawnIslands {
            color,
            op,
            count,
        }))
    }

    pub(crate) fn parse_variable_domain(&mut self) -> Result<VariableDomain, ParseError> {
        let pos = self.current_pos();
        if let Some(Token::LBracket) = self.peek() {
            let (sqs, pcs) = self.parse_square_or_piece_set()?;
            if let Some(pcs) = pcs {
                if let Some(first) = pcs.first() {
                    match first {
                        SquareContent::Piece(p) => {
                            return Ok(VariableDomain::Piece(PieceMatcher::new(
                                Some(p.color),
                                Some(p.role),
                            )))
                        }
                        SquareContent::Role(r) => {
                            return Ok(VariableDomain::Piece(PieceMatcher::new(None, Some(*r))))
                        }
                        SquareContent::Color(c) => {
                            return Ok(VariableDomain::Piece(PieceMatcher::new(Some(*c), None)))
                        }
                        SquareContent::Occupied => return Ok(VariableDomain::AnyPiece),
                        _ => {}
                    }
                }
            }
            if let Some(sqs) = sqs {
                return Ok(VariableDomain::SquareSet(sqs));
            }
        }

        if let Some(Token::Ident(ref s)) = self.peek() {
            let s_clone = s.clone();
            let s_low = s_clone.to_lowercase();
            if s_low == "any" || s_low == "piece" || s_low == "occupied" || s == "Aa" || s == "[Aa]"
            {
                self.advance();
                return Ok(VariableDomain::AnyPiece);
            }
            if let Some((c, r)) = parse_piece_specifier(&s_clone) {
                self.advance();
                return Ok(VariableDomain::Piece(PieceMatcher::new(c, r)));
            }
            if let Ok(sq) = Square::from_str(&s_low) {
                self.advance();
                return Ok(VariableDomain::SquareSet(vec![sq]));
            }
        }

        Err(ParseError::new(
            "Expected piece type (e.g. N, bp, [RNB]), piece set, or square set for variable domain"
                .to_string(),
            pos,
        ))
    }

    pub(crate) fn parse_castling_filter(&mut self) -> Result<SearchQuery, ParseError> {
        let mut color = Color::White;
        let mut kingside = None;
        let mut queenside = None;

        if matches!(self.peek(), Some(Token::LParen) | Some(Token::LBracket)) {
            let is_bracket = matches!(self.peek(), Some(Token::LBracket));
            self.advance();
            while let Some(tok) = self.peek() {
                if (is_bracket && matches!(tok, Token::RBracket))
                    || (!is_bracket && matches!(tok, Token::RParen))
                {
                    self.advance();
                    break;
                }
                if let Token::Comma = tok {
                    self.advance();
                    continue;
                }
                if let Token::Ident(s) = tok {
                    let s_low = s.to_lowercase();
                    self.advance();
                    match s_low.as_str() {
                        "white" | "w" => color = Color::White,
                        "black" | "b" => color = Color::Black,
                        "kingside" | "k" | "o-o" | "oo" | "short" => kingside = Some(true),
                        "queenside" | "q" | "o-o-o" | "ooo" | "long" => queenside = Some(true),
                        "both" | "any" | "all" => {
                            kingside = Some(true);
                            queenside = Some(true);
                        }
                        "none" | "no" => {
                            kingside = Some(false);
                            queenside = Some(false);
                        }
                        _ => {}
                    }
                } else {
                    self.advance();
                }
            }
        } else {
            // Unbracketed: castling white kingside
            while let Some(Token::Ident(s)) = self.peek() {
                let s_low = s.to_lowercase();
                match s_low.as_str() {
                    "white" | "w" => {
                        self.advance();
                        color = Color::White;
                    }
                    "black" | "b" => {
                        self.advance();
                        color = Color::Black;
                    }
                    "kingside" | "k" | "o-o" | "oo" | "short" => {
                        self.advance();
                        kingside = Some(true);
                    }
                    "queenside" | "q" | "o-o-o" | "ooo" | "long" => {
                        self.advance();
                        queenside = Some(true);
                    }
                    "both" | "any" | "all" => {
                        self.advance();
                        kingside = Some(true);
                        queenside = Some(true);
                    }
                    _ => break,
                }
            }
        }

        if kingside.is_none() && queenside.is_none() {
            kingside = Some(true);
        }

        Ok(SearchQuery::Position(PositionPattern::Castling {
            color,
            kingside,
            queenside,
        }))
    }
}
