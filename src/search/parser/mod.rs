use shakmaty::{Color, Piece, Role, Square};
use std::str::FromStr;

pub mod headers;
pub mod helpers;
pub mod lexer;
pub mod moves;
pub mod pieces;
pub mod ranges;
pub mod squares;
pub mod tactics;
pub mod validator;

pub use helpers::{
    expand_diagonal_ray, expand_rectangular_range, expand_square_specifier, parse_piece_specifier,
    parse_square_or_piece,
};
pub use lexer::{is_ident_char, is_ident_start, Lexer, ParseError, Token};
pub use validator::validate_query_semantics;

use super::query::{
    BoardMutation, ComparisonOp, MaterialPredicate, PositionPattern, SearchQuery, SquareContent,
};

/// CQL-Lite query parser that translates textual search expressions into a SearchQuery AST
pub struct QueryParser<'a> {
    tokens: &'a [(usize, Token)],
    pos: usize,
}

impl<'a> QueryParser<'a> {
    pub fn new(tokens: &'a [(usize, Token)]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse a complete search query string with rich error diagnostics
    pub fn parse_str(input: &str) -> Result<SearchQuery, ParseError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().map_err(|e| e.with_source_context(input))?;
        if tokens.is_empty() {
            return Ok(SearchQuery::And(Vec::new()));
        }
        let mut parser = QueryParser::new(&tokens);
        let query = parser
            .parse_or_expr()
            .map_err(|e| e.with_source_context(input))?;
        if parser.pos < parser.tokens.len() {
            let (pos, tok) = &parser.tokens[parser.pos];
            let err = ParseError::new(
                format!("Unexpected trailing token: {:?}", tok),
                *pos,
            )
            .with_help("Make sure each condition or keyword is separated by newlines, 'and', or parentheses.")
            .with_source_context(input);
            return Err(err);
        }
        validate_query_semantics(&query, 0).map_err(|e| e.with_source_context(input))?;
        Ok(query)
    }

    /// Parse and explain the query, expanding symmetries and breaking down branches
    pub fn explain(input: &str) -> Result<super::explain::QueryExplanation, ParseError> {
        let query = Self::parse_str(input)?;
        Ok(super::explain::explain_query(input, &query))
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(_, t)| t)
    }

    pub(crate) fn peek_nth(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n).map(|(_, t)| t)
    }

    pub(crate) fn current_pos(&self) -> usize {
        self.tokens.get(self.pos).map(|(p, _)| *p).unwrap_or(0)
    }

    pub(crate) fn advance(&mut self) -> Option<&Token> {
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos].1;
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    pub(crate) fn match_ident(&mut self, expected: &str) -> bool {
        if let Some(Token::Ident(s)) = self.peek() {
            if s.eq_ignore_ascii_case(expected) {
                self.advance();
                return true;
            }
        }
        false
    }

    pub(crate) fn expect_ident(&mut self) -> Result<String, ParseError> {
        let pos = self.current_pos();
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s.clone()),
            Some(tok) => Err(ParseError::new(
                format!("Expected identifier, found {:?}", tok),
                pos,
            )),
            None => Err(ParseError::new(
                "Expected identifier, found EOF".to_string(),
                pos,
            )),
        }
    }

    pub(crate) fn expect_string_or_ident(&mut self) -> Result<String, ParseError> {
        let pos = self.current_pos();
        match self.advance() {
            Some(Token::StringLit(s)) => Ok(s.clone()),
            Some(Token::Ident(s)) => Ok(s.clone()),
            Some(tok) => Err(ParseError::new(
                format!("Expected string or identifier, found {:?}", tok),
                pos,
            )),
            None => Err(ParseError::new(
                "Expected string or identifier, found EOF".to_string(),
                pos,
            )),
        }
    }

    pub(crate) fn expect_number(&mut self) -> Result<i64, ParseError> {
        let pos = self.current_pos();
        match self.advance() {
            Some(Token::Number(n)) => Ok(*n),
            Some(tok) => Err(ParseError::new(
                format!("Expected number, found {:?}", tok),
                pos,
            )),
            None => Err(ParseError::new(
                "Expected number, found EOF".to_string(),
                pos,
            )),
        }
    }

    pub(crate) fn expect_token(&mut self, expected: Token) -> Result<(), ParseError> {
        let pos = self.current_pos();
        match self.advance() {
            Some(tok) if *tok == expected => Ok(()),
            Some(tok) => Err(ParseError::new(
                format!("Expected {:?}, found {:?}", expected, tok),
                pos,
            )),
            None => Err(ParseError::new(
                format!("Expected {:?}, found EOF", expected),
                pos,
            )),
        }
    }

    pub(crate) fn parse_comparison_op(&mut self) -> ComparisonOp {
        if let Some(Token::Colon) = self.peek() {
            self.advance();
            return ComparisonOp::Equal;
        }
        if let Some(Token::Eq) = self.peek() {
            self.advance();
            return ComparisonOp::Equal;
        }
        if let Some(Token::Neq) = self.peek() {
            self.advance();
            return ComparisonOp::NotEqual;
        }
        if let Some(Token::Gte) = self.peek() {
            self.advance();
            return ComparisonOp::GreaterThanOrEqual;
        }
        if let Some(Token::Gt) = self.peek() {
            self.advance();
            return ComparisonOp::GreaterThan;
        }
        if let Some(Token::Lte) = self.peek() {
            self.advance();
            return ComparisonOp::LessThanOrEqual;
        }
        if let Some(Token::Lt) = self.peek() {
            self.advance();
            return ComparisonOp::LessThan;
        }

        ComparisonOp::Equal
    }

    pub(crate) fn parse_string_comparison_op(&mut self) -> ComparisonOp {
        if self.match_ident("contains") || self.match_ident("has") {
            return ComparisonOp::Contains;
        }
        if self.match_ident("startswith") || self.match_ident("starts_with") {
            return ComparisonOp::StartsWith;
        }
        if self.match_ident("endswith") || self.match_ident("ends_with") {
            return ComparisonOp::EndsWith;
        }
        if self.match_ident("matches") || self.match_ident("regex") {
            return ComparisonOp::Regex;
        }
        if let Some(Token::Tilde) = self.peek() {
            self.advance();
            return ComparisonOp::Regex;
        }
        if let Some(Token::Eq) = self.peek() {
            self.advance();
            if let Some(Token::Tilde) = self.peek() {
                self.advance();
                return ComparisonOp::Regex;
            }
            return ComparisonOp::Equal;
        }
        if let Some(Token::Colon) = self.peek() {
            self.advance();
            if let Some(Token::Tilde) = self.peek() {
                self.advance();
                return ComparisonOp::Regex;
            }
            return ComparisonOp::Contains;
        }
        if let Some(Token::Neq) = self.peek() {
            self.advance();
            return ComparisonOp::NotEqual;
        }
        if let Some(Token::Gte) = self.peek() {
            self.advance();
            return ComparisonOp::GreaterThanOrEqual;
        }
        if let Some(Token::Gt) = self.peek() {
            self.advance();
            return ComparisonOp::GreaterThan;
        }
        if let Some(Token::Lte) = self.peek() {
            self.advance();
            return ComparisonOp::LessThanOrEqual;
        }
        if let Some(Token::Lt) = self.peek() {
            self.advance();
            return ComparisonOp::LessThan;
        }

        ComparisonOp::Contains
    }

    pub(crate) fn parse_string_or_regex_val(
        &mut self,
        default_op: ComparisonOp,
    ) -> Result<(String, ComparisonOp), ParseError> {
        if self.match_ident("regex") {
            if let Some(Token::LParen) = self.peek() {
                self.advance();
                let pattern = self.expect_string_or_ident()?;
                if let Some(Token::RParen) = self.peek() {
                    self.advance();
                    return Ok((pattern, ComparisonOp::Regex));
                }
                return Err(ParseError::new(
                    "Expected ')' after regex pattern".to_string(),
                    self.current_pos(),
                ));
            }
        }
        let val = self.expect_string_or_ident()?;
        Ok((val, default_op))
    }

    // Grammar:
    // or_expr  ::= and_expr ("or" and_expr)*
    // and_expr ::= unary_expr ("and"? unary_expr)*
    // unary_expr ::= "not" unary_expr | primary_expr
    // primary_expr ::= cql_wrapper | header_pred | pos_pred | pawn_pred | path_pred | material_pred | ply_range | "(" or_expr ")"

    pub(crate) fn parse_or_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let mut left = self.parse_and_expr()?;

        while self.match_ident("or") {
            let right = self.parse_and_expr()?;
            left = match left {
                SearchQuery::Or(mut list) => {
                    list.push(right);
                    SearchQuery::Or(list)
                }
                other => SearchQuery::Or(vec![other, right]),
            };
        }

        Ok(left)
    }

    pub(crate) fn parse_and_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let mut list = Vec::new();
        list.push(self.parse_unary_expr()?);

        while self.has_more_in_and() {
            self.match_ident("and"); // Optional 'and'
            list.push(self.parse_unary_expr()?);
        }

        if list.len() == 1 {
            Ok(list.into_iter().next().unwrap())
        } else {
            Ok(SearchQuery::And(list))
        }
    }

    fn has_more_in_and(&self) -> bool {
        if let Some(tok) = self.peek() {
            match tok {
                Token::RParen | Token::RBracket | Token::RBrace => false,
                Token::Ident(s) if s.eq_ignore_ascii_case("or") => false,
                _ => true,
            }
        } else {
            false
        }
    }

    pub(crate) fn parse_unary_expr(&mut self) -> Result<SearchQuery, ParseError> {
        if self.match_ident("not") {
            let expr = self.parse_unary_expr()?;
            return Ok(SearchQuery::Not(Box::new(expr)));
        }
        self.parse_primary_expr()
    }

    pub(crate) fn parse_primary_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();

        // 1. Parenthesized expression: `( ... )`
        if let Some(Token::LParen) = self.peek() {
            let saved_pos = self.pos;
            if let Ok(sq_expr) = self.parse_square_set_query() {
                return Ok(sq_expr);
            }
            self.pos = saved_pos;
            self.advance();
            let expr = self.parse_or_expr()?;
            if let Some(Token::RParen) = self.peek() {
                self.advance();
                return Ok(expr);
            }
            return Err(ParseError::new(
                "Expected ')'".to_string(),
                self.current_pos(),
            ));
        }

        // 1b. Tilde: `~occupied count >= 32`
        if let Some(Token::Tilde) = self.peek() {
            return self.parse_square_set_query();
        }

        // 2. CQL Wrapper: `cql ( ... )`
        if self.match_ident("cql") {
            if let Some(Token::LParen) = self.peek() {
                self.advance();
                let expr = self.parse_or_expr()?;
                if let Some(Token::RParen) = self.peek() {
                    self.advance();
                    return Ok(expr);
                }
                return Err(ParseError::new(
                    "Expected ')' after cql".to_string(),
                    self.current_pos(),
                ));
            }
        }

        // 3. Ply Range / Ply check: `ply 1..20 ( ... )` or `ply == 10` or `range 0..10 ( ... )`
        if self.match_ident("ply") || self.match_ident("range") {
            return self.parse_ply_expr();
        }

        // 4. Bracketed lists: piece lists (e.g. `[B, b] >= 1`, `[Q, R] on [d1, e1]`) or square sets (`[a1, a2]`)
        if let Some(Token::LBracket) = self.peek() {
            if !self.is_bracket_piece_list() || self.has_square_set_operator_ahead() {
                let saved_pos = self.pos;
                if let Ok(sq) = self.parse_square_set_query() {
                    return Ok(sq);
                }
                self.pos = saved_pos;
            }
            return self.parse_piece_on_square();
        }

        // 4. Ident-based keywords
        let ident_opt = if let Some(Token::Ident(s)) = self.peek() {
            Some(s.clone())
        } else {
            None
        };

        if let Some(ident_str) = ident_opt {
            let key = ident_str.to_lowercase();
            match key.as_str() {
                "movenumber" | "move_number" | "movenum" | "move_num" => {
                    self.advance();
                    return self.parse_move_number_expr();
                }
                "player" | "white" | "black" | "elo" | "anyelo" | "any_elo" | "whiteelo"
                | "white_elo" | "blackelo" | "black_elo" | "avgelo" | "avg_elo" | "elodiff"
                | "elo_diff" | "raw_elodiff" | "raw_elo_diff" | "rawelodiff" | "result" | "eco"
                | "date" | "year" | "event" | "site" | "tag" | "header" => {
                    return self.parse_header_keyword(&key).map(|opt| opt.unwrap());
                }

                // Positions & Board States
                "fen" | "position" | "pos" | "board" => {
                    self.advance();
                    let _ = self.parse_comparison_op();
                    let fen_str = self.expect_string_or_ident()?;
                    return Ok(SearchQuery::Position(PositionPattern::ExactFen(fen_str)));
                }
                "placement" => {
                    self.advance();
                    let _ = self.parse_comparison_op();
                    let placement_str = self.expect_string_or_ident()?;
                    return Ok(SearchQuery::Position(PositionPattern::PiecePlacement(
                        placement_str,
                    )));
                }
                "check" => {
                    self.advance();
                    return Ok(SearchQuery::Position(PositionPattern::BoardState {
                        is_check: Some(true),
                        is_checkmate: None,
                        is_stalemate: None,
                    }));
                }
                "mate" | "checkmate" => {
                    self.advance();
                    return Ok(SearchQuery::Position(PositionPattern::BoardState {
                        is_check: None,
                        is_checkmate: Some(true),
                        is_stalemate: None,
                    }));
                }
                "stalemate" => {
                    self.advance();
                    return Ok(SearchQuery::Position(PositionPattern::BoardState {
                        is_check: None,
                        is_checkmate: None,
                        is_stalemate: Some(true),
                    }));
                }
                "piece" => {
                    if self
                        .peek_nth(1)
                        .is_some_and(|t| matches!(t, Token::Ident(s) if s.starts_with('$')))
                    {
                        self.advance();
                        let var_name = self.expect_ident()?;
                        self.match_ident("in");
                        let domain = self.parse_variable_domain()?;
                        self.expect_token(Token::LBrace)?;
                        let sub_query = self.parse_or_expr()?;
                        self.expect_token(Token::RBrace)?;
                        return Ok(SearchQuery::VariableBinding {
                            var_name,
                            domain,
                            query: Box::new(sub_query),
                        });
                    }
                    self.advance();
                    return self.parse_piece_on_square();
                }
                "square" => {
                    self.advance();
                    return self.parse_piece_on_square();
                }
                "white_pieces" | "white_piece" => {
                    if self.has_square_set_operator_ahead() {
                        let saved_pos = self.pos;
                        if let Ok(sq) = self.parse_square_set_query() {
                            return Ok(sq);
                        }
                        self.pos = saved_pos;
                    }
                    self.advance();
                    return self.parse_piece_count_or_squares(SquareContent::Color(Color::White));
                }
                "black_pieces" | "black_piece" => {
                    if self.has_square_set_operator_ahead() {
                        let saved_pos = self.pos;
                        if let Ok(sq) = self.parse_square_set_query() {
                            return Ok(sq);
                        }
                        self.pos = saved_pos;
                    }
                    self.advance();
                    return self.parse_piece_count_or_squares(SquareContent::Color(Color::Black));
                }
                "occupied" | "any_piece" => {
                    if self.has_square_set_operator_ahead() {
                        let saved_pos = self.pos;
                        if let Ok(sq) = self.parse_square_set_query() {
                            return Ok(sq);
                        }
                        self.pos = saved_pos;
                    }
                    self.advance();
                    return self.parse_piece_count_or_squares(SquareContent::Occupied);
                }
                "empty" => {
                    if self.has_square_set_operator_ahead() {
                        let saved_pos = self.pos;
                        if let Ok(sq) = self.parse_square_set_query() {
                            return Ok(sq);
                        }
                        self.pos = saved_pos;
                    }
                    self.advance();
                    return self.parse_piece_count_or_squares(SquareContent::Empty);
                }
                "wtm" => {
                    self.advance();
                    return Ok(SearchQuery::Position(PositionPattern::Turn(Color::White)));
                }
                "btm" => {
                    self.advance();
                    return Ok(SearchQuery::Position(PositionPattern::Turn(Color::Black)));
                }
                "turn" => {
                    self.advance();
                    let _ = self.parse_comparison_op();
                    let color_str = self.expect_ident()?;
                    let color = match color_str.to_lowercase().as_str() {
                        "white" | "w" => Color::White,
                        "black" | "b" => Color::Black,
                        _ => {
                            return Err(ParseError::new(
                                format!("Invalid turn color: {}", color_str),
                                pos,
                            ))
                        }
                    };
                    return Ok(SearchQuery::Position(PositionPattern::Turn(color)));
                }

                // Pawn Structures
                "passedpawns" | "passed_pawns" | "passed" => {
                    self.advance();
                    return self.parse_pawn_pred_passed();
                }
                "isolatedpawns" | "isolated_pawns" | "isolated" => {
                    self.advance();
                    return self.parse_pawn_pred_isolated();
                }
                "doubledpawns" | "doubled_pawns" | "doubled" => {
                    self.advance();
                    return self.parse_pawn_pred_doubled();
                }
                "backwardpawns" | "backward_pawns" | "backward" => {
                    self.advance();
                    return self.parse_pawn_pred_backward();
                }
                "pawnislands" | "pawn_islands" | "islands" => {
                    self.advance();
                    return self.parse_pawn_pred_islands();
                }

                "castling" | "castle_rights" | "castling_rights" => {
                    self.advance();
                    return self.parse_castling_filter();
                }

                // Moves & Legal Moves & Paths
                "legal" => {
                    self.advance();
                    return self.parse_move_filter(true);
                }
                "move" => {
                    self.advance();
                    return self.parse_move_filter(false);
                }
                "prev" | "previous" => {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Ident(ref s)) if s.eq_ignore_ascii_case("move"))
                    {
                        self.advance();
                    }
                    let mut q = self.parse_move_filter(false)?;
                    if let SearchQuery::Move(ref mut m) = q {
                        m.is_previous = true;
                    }
                    return Ok(q);
                }
                "line" | "cqlline" | "cql_line" => {
                    self.advance();
                    let single_color =
                        if self.match_ident("singlecolor") || self.match_ident("single_color") {
                            Some(None)
                        } else if self.match_ident("white") {
                            Some(Some(Color::White))
                        } else if self.match_ident("black") {
                            Some(Some(Color::Black))
                        } else {
                            None
                        };
                    return self.parse_cql_line_or_legacy_path(single_color);
                }
                "path" => {
                    self.advance();
                    let single_color =
                        if self.match_ident("singlecolor") || self.match_ident("single_color") {
                            Some(None)
                        } else if self.match_ident("white") {
                            Some(Some(Color::White))
                        } else if self.match_ident("black") {
                            Some(Some(Color::Black))
                        } else {
                            None
                        };
                    return self.parse_path_expr(false, single_color);
                }
                "cql_path" | "cqlpath" | "turnstile" | "sequence" | "seq" => {
                    self.advance();
                    let single_color =
                        if self.match_ident("singlecolor") || self.match_ident("single_color") {
                            Some(None)
                        } else if self.match_ident("white") {
                            Some(Some(Color::White))
                        } else if self.match_ident("black") {
                            Some(Some(Color::Black))
                        } else {
                            None
                        };
                    return self.parse_cql_path_expr(single_color);
                }

                // Material
                "opposite_bishops" => {
                    self.advance();
                    return Ok(SearchQuery::Material(MaterialPredicate {
                        opposite_bishops: Some(true),
                        ..Default::default()
                    }));
                }
                "same_colored_bishops" => {
                    self.advance();
                    return Ok(SearchQuery::Material(MaterialPredicate {
                        same_colored_bishops: Some(true),
                        ..Default::default()
                    }));
                }
                // Light / Dark square-color modifiers for ANY piece, piece group, or square counts
                "light" | "light_squares" | "light_pieces" | "light_piece" => {
                    self.advance();
                    return self.parse_color_modified_piece(true);
                }
                "dark" | "dark_squares" | "dark_pieces" | "dark_piece" => {
                    self.advance();
                    return self.parse_color_modified_piece(false);
                }
                "light_bishops" | "light_bishop" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content: SquareContent::Role(shakmaty::Role::Bishop),
                        squares: Some(shakmaty::Bitboard::LIGHT_SQUARES.into_iter().collect()),
                        op,
                        count,
                    }));
                }
                "dark_bishops" | "dark_bishop" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content: SquareContent::Role(shakmaty::Role::Bishop),
                        squares: Some(shakmaty::Bitboard::DARK_SQUARES.into_iter().collect()),
                        op,
                        count,
                    }));
                }
                "white_light_bishops" | "white_light_bishop" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content: SquareContent::Piece(shakmaty::Piece {
                            color: shakmaty::Color::White,
                            role: shakmaty::Role::Bishop,
                        }),
                        squares: Some(shakmaty::Bitboard::LIGHT_SQUARES.into_iter().collect()),
                        op,
                        count,
                    }));
                }
                "white_dark_bishops" | "white_dark_bishop" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content: SquareContent::Piece(shakmaty::Piece {
                            color: shakmaty::Color::White,
                            role: shakmaty::Role::Bishop,
                        }),
                        squares: Some(shakmaty::Bitboard::DARK_SQUARES.into_iter().collect()),
                        op,
                        count,
                    }));
                }
                "black_light_bishops" | "black_light_bishop" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content: SquareContent::Piece(shakmaty::Piece {
                            color: shakmaty::Color::Black,
                            role: shakmaty::Role::Bishop,
                        }),
                        squares: Some(shakmaty::Bitboard::LIGHT_SQUARES.into_iter().collect()),
                        op,
                        count,
                    }));
                }
                "black_dark_bishops" | "black_dark_bishop" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let count = self.expect_number()? as usize;
                    return Ok(SearchQuery::Position(PositionPattern::PieceCount {
                        content: SquareContent::Piece(shakmaty::Piece {
                            color: shakmaty::Color::Black,
                            role: shakmaty::Role::Bishop,
                        }),
                        squares: Some(shakmaty::Bitboard::DARK_SQUARES.into_iter().collect()),
                        op,
                        count,
                    }));
                }

                // Power & Piece Material Points
                "white_power" | "whitepower" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    if let Some(Token::Ident(s)) = self.peek() {
                        let s_low = s.to_lowercase();
                        if s_low == "black_power" || s_low == "blackpower" || s_low == "black" {
                            self.advance();
                            return Ok(SearchQuery::Power(
                                super::query::PowerPredicate::WhiteVsBlackPower { op },
                            ));
                        }
                    }
                    let val = self.expect_number()? as i32;
                    return Ok(SearchQuery::Power(
                        super::query::PowerPredicate::WhitePower { op, value: val },
                    ));
                }
                "black_power" | "blackpower" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    if let Some(Token::Ident(s)) = self.peek() {
                        let s_low = s.to_lowercase();
                        if s_low == "white_power" || s_low == "whitepower" || s_low == "white" {
                            self.advance();
                            return Ok(SearchQuery::Power(
                                super::query::PowerPredicate::WhiteVsBlackPower { op: op.invert() },
                            ));
                        }
                    }
                    let val = self.expect_number()? as i32;
                    return Ok(SearchQuery::Power(
                        super::query::PowerPredicate::BlackPower { op, value: val },
                    ));
                }
                "power" | "total_power" | "totalpower" => {
                    self.advance();
                    if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let target = self.expect_ident()?.to_lowercase();
                        if let Some(Token::RParen) = self.peek() {
                            self.advance();
                        }
                        let op = self.parse_comparison_op();
                        if target == "white" || target == "w" {
                            if let Some(Token::Ident(s)) = self.peek() {
                                let s_low = s.to_lowercase();
                                if s_low == "black_power"
                                    || s_low == "blackpower"
                                    || s_low == "black"
                                {
                                    self.advance();
                                    return Ok(SearchQuery::Power(
                                        super::query::PowerPredicate::WhiteVsBlackPower { op },
                                    ));
                                }
                            }
                            let val = self.expect_number()? as i32;
                            return Ok(SearchQuery::Power(
                                super::query::PowerPredicate::WhitePower { op, value: val },
                            ));
                        } else if target == "black" || target == "b" {
                            if let Some(Token::Ident(s)) = self.peek() {
                                let s_low = s.to_lowercase();
                                if s_low == "white_power"
                                    || s_low == "whitepower"
                                    || s_low == "white"
                                {
                                    self.advance();
                                    return Ok(SearchQuery::Power(
                                        super::query::PowerPredicate::WhiteVsBlackPower {
                                            op: op.invert(),
                                        },
                                    ));
                                }
                            }
                            let val = self.expect_number()? as i32;
                            return Ok(SearchQuery::Power(
                                super::query::PowerPredicate::BlackPower { op, value: val },
                            ));
                        }
                    }
                    let op = self.parse_comparison_op();
                    let val = self.expect_number()? as i32;
                    return Ok(SearchQuery::Power(
                        super::query::PowerPredicate::TotalPower { op, value: val },
                    ));
                }
                "power_diff"
                | "power_difference"
                | "powerdiff"
                | "material_diff"
                | "material_difference"
                | "materialdiff" => {
                    self.advance();
                    let op = self.parse_comparison_op();
                    let val = self.expect_number()? as i32;
                    return Ok(SearchQuery::Power(
                        super::query::PowerPredicate::PowerDifference {
                            op,
                            value: val,
                            absolute: true,
                        },
                    ));
                }

                // Tactical & Geometric Motifs
                "pin" => {
                    self.advance();
                    return self.parse_pin_expr();
                }
                "fork" => {
                    self.advance();
                    return self.parse_fork_expr();
                }
                "discovered_attack" => {
                    self.advance();
                    let color = self.parse_color_token();
                    return Ok(SearchQuery::Tactical(
                        crate::search::query::TacticalPredicate::DiscoveredAttack {
                            color,
                            is_check: false,
                        },
                    ));
                }
                "discovered_check" => {
                    self.advance();
                    let color = self.parse_color_token();
                    return Ok(SearchQuery::Tactical(
                        crate::search::query::TacticalPredicate::DiscoveredAttack {
                            color,
                            is_check: true,
                        },
                    ));
                }
                "skewer" => {
                    self.advance();
                    return self.parse_skewer_expr();
                }
                "trapped" => {
                    self.advance();
                    return self.parse_trapped_expr();
                }
                "outpost" => {
                    self.advance();
                    return self.parse_outpost_expr();
                }
                "rook_7th" | "rook_seventh" => {
                    self.advance();
                    let color = self.parse_color_token();
                    return Ok(SearchQuery::Tactical(
                        crate::search::query::TacticalPredicate::RookOnSeventh { color },
                    ));
                }
                "open_file" => {
                    self.advance();
                    return self.parse_open_file_expr(false);
                }
                "semi_open_file" | "semi_open" => {
                    self.advance();
                    return self.parse_open_file_expr(true);
                }
                "distance" => {
                    self.advance();
                    return self.parse_distance_expr();
                }
                "attacks" | "attack" => {
                    self.advance();
                    return self.parse_attacks_expr();
                }
                "attacked" | "is_attacked" | "attacked_by" => {
                    self.advance();
                    return self.parse_attacked_expr();
                }
                "cql" => {
                    self.advance();
                    if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        while let Some(tok) = self.peek() {
                            if let Token::RParen = tok {
                                self.advance();
                                break;
                            }
                            self.advance();
                        }
                    }
                    return self.parse_or_expr();
                }
                "occurrences" | "occurrence" => {
                    self.advance();
                    return self.parse_occurrences_expr();
                }
                "flipcolor" | "flip_color" | "invertcolor" | "invert_color" | "fliphorizontal"
                | "flip_horizontal" | "flip_h" | "flipvertical" | "flip_vertical" | "flip_v"
                | "flipmaindiagonal" | "flip_main_diagonal" | "flip_diag" | "flipdiagonal"
                | "flip_diagonal" | "flipantidiagonal" | "flip_anti_diagonal" | "flip_antidiag"
                | "rotate90" | "rotate_90" | "rot90" | "rotate180" | "rotate_180" | "rot180"
                | "rotate270" | "rotate_270" | "rot270" | "rotate" | "rot" | "all_rotations" => {
                    let sym = match key.as_str() {
                        "flipcolor" | "flip_color" | "invertcolor" | "invert_color" => {
                            crate::search::transform::BoardSymmetry::ColorInvert
                        }
                        "fliphorizontal" | "flip_horizontal" | "flip_h" => {
                            crate::search::transform::BoardSymmetry::HorizontalMirror
                        }
                        "flipvertical" | "flip_vertical" | "flip_v" => {
                            crate::search::transform::BoardSymmetry::VerticalMirror
                        }
                        "flipmaindiagonal" | "flip_main_diagonal" | "flip_diag"
                        | "flipdiagonal" | "flip_diagonal" => {
                            crate::search::transform::BoardSymmetry::MainDiagonal
                        }
                        "flipantidiagonal" | "flip_anti_diagonal" | "flip_antidiag" => {
                            crate::search::transform::BoardSymmetry::AntiDiagonal
                        }
                        "rotate90" | "rotate_90" | "rot90" | "rotate" | "rot" | "all_rotations" => {
                            crate::search::transform::BoardSymmetry::AllRotations
                        }
                        "rotate180" | "rotate_180" | "rot180" => {
                            crate::search::transform::BoardSymmetry::Rotate180
                        }
                        "rotate270" | "rotate_270" | "rot270" => {
                            crate::search::transform::BoardSymmetry::Rotate270
                        }
                        _ => unreachable!(),
                    };
                    self.advance();
                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RParen) = self.peek() {
                            self.advance();
                        }
                        q
                    } else {
                        self.parse_primary_expr()?
                    };
                    return Ok(SearchQuery::Symmetric {
                        query: Box::new(sub_query),
                        symmetry: sym,
                    });
                }
                "symmetry" | "flip" => {
                    self.advance();
                    let mut sym = if key == "flip" {
                        crate::search::transform::BoardSymmetry::HorizontalMirror
                    } else {
                        crate::search::transform::BoardSymmetry::AnySpatialSymmetry
                    };

                    if matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq)) {
                        self.advance();
                        let mode_str = self.expect_ident()?;
                        sym = match mode_str.to_lowercase().as_str() {
                            "horizontal" | "h" | "lr" => {
                                crate::search::transform::BoardSymmetry::HorizontalMirror
                            }
                            "vertical" | "v" | "ud" => {
                                crate::search::transform::BoardSymmetry::VerticalMirror
                            }
                            "maindiagonal" | "main_diagonal" | "diagonal" | "diag" => {
                                crate::search::transform::BoardSymmetry::MainDiagonal
                            }
                            "antidiagonal" | "anti_diagonal" | "antidiag" => {
                                crate::search::transform::BoardSymmetry::AntiDiagonal
                            }
                            "rotate" | "rot" | "180" | "rotate180" => {
                                crate::search::transform::BoardSymmetry::Rotate180
                            }
                            "90" | "rotate90" => crate::search::transform::BoardSymmetry::Rotate90,
                            "270" | "rotate270" => {
                                crate::search::transform::BoardSymmetry::Rotate270
                            }
                            "color" | "c" => crate::search::transform::BoardSymmetry::ColorInvert,
                            "color_horizontal" => {
                                crate::search::transform::BoardSymmetry::ColorInvertHorizontal
                            }
                            "spatial" => {
                                crate::search::transform::BoardSymmetry::AnySpatialSymmetry
                            }
                            "any" | "all" => {
                                crate::search::transform::BoardSymmetry::AnyTotalSymmetry
                            }
                            _ => {
                                return Err(ParseError::new(
                                    format!("Unknown symmetry mode: {}", mode_str),
                                    pos,
                                ));
                            }
                        };
                    } else if let Some(Token::Ident(ref id)) = self.peek() {
                        let id_low = id.to_lowercase();
                        let matched_sym = match id_low.as_str() {
                            "horizontal" | "h" | "lr" => {
                                Some(crate::search::transform::BoardSymmetry::HorizontalMirror)
                            }
                            "vertical" | "v" | "ud" => {
                                Some(crate::search::transform::BoardSymmetry::VerticalMirror)
                            }
                            "maindiagonal" | "main_diagonal" | "diagonal" | "diag" => {
                                Some(crate::search::transform::BoardSymmetry::MainDiagonal)
                            }
                            "antidiagonal" | "anti_diagonal" | "antidiag" => {
                                Some(crate::search::transform::BoardSymmetry::AntiDiagonal)
                            }
                            "rotate" | "rot" | "180" | "rotate180" => {
                                Some(crate::search::transform::BoardSymmetry::Rotate180)
                            }
                            "90" | "rotate90" => {
                                Some(crate::search::transform::BoardSymmetry::Rotate90)
                            }
                            "270" | "rotate270" => {
                                Some(crate::search::transform::BoardSymmetry::Rotate270)
                            }
                            "color" | "c" => {
                                Some(crate::search::transform::BoardSymmetry::ColorInvert)
                            }
                            "color_horizontal" => {
                                Some(crate::search::transform::BoardSymmetry::ColorInvertHorizontal)
                            }
                            "spatial" => {
                                Some(crate::search::transform::BoardSymmetry::AnySpatialSymmetry)
                            }
                            "any" | "all" => {
                                Some(crate::search::transform::BoardSymmetry::AnyTotalSymmetry)
                            }
                            _ => None,
                        };
                        if let Some(s) = matched_sym {
                            self.advance();
                            sym = s;
                        }
                    }

                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RParen) = self.peek() {
                            self.advance();
                        }
                        q
                    } else {
                        self.parse_primary_expr()?
                    };
                    return Ok(SearchQuery::Symmetric {
                        query: Box::new(sub_query),
                        symmetry: sym,
                    });
                }
                "shifthorizontal" | "shift_horizontal" | "shift_h" | "shiftvertical"
                | "shift_vertical" | "shift_v" | "shift" | "shift_all" | "shiftall" => {
                    self.advance();
                    let mut mode = match key.as_str() {
                        "shifthorizontal" | "shift_horizontal" | "shift_h" => {
                            crate::search::transform::ShiftMode::Horizontal
                        }
                        "shiftvertical" | "shift_vertical" | "shift_v" => {
                            crate::search::transform::ShiftMode::Vertical
                        }
                        _ => crate::search::transform::ShiftMode::All,
                    };

                    if matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq)) {
                        self.advance();
                        let mode_str = self.expect_ident()?;
                        mode = match mode_str.to_lowercase().as_str() {
                            "horizontal" | "h" | "files" => {
                                crate::search::transform::ShiftMode::Horizontal
                            }
                            "vertical" | "v" | "ranks" => {
                                crate::search::transform::ShiftMode::Vertical
                            }
                            "all" | "any" | "both" => crate::search::transform::ShiftMode::All,
                            _ => {
                                return Err(ParseError::new(
                                    format!("Unknown shift mode: {}", mode_str),
                                    pos,
                                ));
                            }
                        };
                    } else if let Some(Token::Ident(ref id)) = self.peek() {
                        let id_low = id.to_lowercase();
                        let matched_mode = match id_low.as_str() {
                            "horizontal" | "h" | "files" => {
                                Some(crate::search::transform::ShiftMode::Horizontal)
                            }
                            "vertical" | "v" | "ranks" => {
                                Some(crate::search::transform::ShiftMode::Vertical)
                            }
                            "all" | "any" | "both" => {
                                Some(crate::search::transform::ShiftMode::All)
                            }
                            _ => None,
                        };
                        if let Some(m) = matched_mode {
                            self.advance();
                            mode = m;
                        }
                    }

                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RParen) = self.peek() {
                            self.advance();
                        }
                        q
                    } else {
                        self.parse_primary_expr()?
                    };
                    return Ok(SearchQuery::Shift {
                        mode,
                        query: Box::new(sub_query),
                    });
                }
                "initial" => {
                    self.advance();
                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RParen) = self.peek() {
                            self.advance();
                        }
                        q
                    } else {
                        self.parse_primary_expr()?
                    };
                    return Ok(SearchQuery::Initial(Box::new(sub_query)));
                }
                "terminal" => {
                    self.advance();
                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        if let Some(Token::RParen) = self.peek() {
                            self.advance();
                        }
                        q
                    } else {
                        self.parse_primary_expr()?
                    };
                    return Ok(SearchQuery::Terminal(Box::new(sub_query)));
                }
                "comment" | "comment_contains" => {
                    self.advance();
                    let op = self.parse_string_comparison_op();
                    let (val, final_op) = self.parse_string_or_regex_val(op)?;
                    let pred = match final_op {
                        ComparisonOp::StartsWith => {
                            crate::search::annotation::CommentPredicate::StartsWith {
                                prefix: val,
                                case_sensitive: false,
                            }
                        }
                        ComparisonOp::Regex => {
                            crate::search::annotation::CommentPredicate::Regex(val)
                        }
                        _ => crate::search::annotation::CommentPredicate::Contains {
                            text: val,
                            case_sensitive: false,
                        },
                    };
                    return Ok(SearchQuery::Annotation(
                        crate::search::annotation::AnnotationPredicate::Comment(pred),
                    ));
                }
                "nag" => {
                    self.advance();
                    let _ = self.parse_comparison_op();
                    if let Some(Token::LBracket) = self.peek() {
                        self.advance();
                        let mut nags = Vec::new();
                        while let Some(tok) = self.peek() {
                            if let Token::RBracket = tok {
                                self.advance();
                                break;
                            }
                            if let Token::Comma = tok {
                                self.advance();
                                continue;
                            }
                            if let Some(Token::Number(n)) = self.peek() {
                                nags.push(*n as u8);
                                self.advance();
                            } else {
                                let s = self.expect_string_or_ident()?;
                                if let Some(code) =
                                    crate::search::annotation::NagPredicate::symbol_to_nag(&s)
                                {
                                    nags.push(code);
                                }
                            }
                        }
                        return Ok(SearchQuery::Annotation(
                            crate::search::annotation::AnnotationPredicate::Nag(
                                crate::search::annotation::NagPredicate::AnyOf(nags),
                            ),
                        ));
                    } else if let Some(Token::Number(n)) = self.peek() {
                        let code = *n as u8;
                        self.advance();
                        return Ok(SearchQuery::Annotation(
                            crate::search::annotation::AnnotationPredicate::Nag(
                                crate::search::annotation::NagPredicate::AnyOf(vec![code]),
                            ),
                        ));
                    } else {
                        let s = self.expect_string_or_ident()?;
                        let code = crate::search::annotation::NagPredicate::symbol_to_nag(&s)
                            .ok_or_else(|| {
                                ParseError::new(format!("Unknown NAG symbol: {}", s), pos)
                            })?;
                        return Ok(SearchQuery::Annotation(
                            crate::search::annotation::AnnotationPredicate::Nag(
                                crate::search::annotation::NagPredicate::AnyOf(vec![code]),
                            ),
                        ));
                    }
                }

                "parent" => {
                    self.advance();
                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        self.expect_token(Token::RBrace)?;
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        self.expect_token(Token::RParen)?;
                        q
                    } else {
                        self.parse_unary_expr()?
                    };
                    return Ok(SearchQuery::Parent(Box::new(sub_query)));
                }

                "child" => {
                    self.advance();
                    let sub_query = if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        self.expect_token(Token::RBrace)?;
                        q
                    } else if let Some(Token::LParen) = self.peek() {
                        self.advance();
                        let q = self.parse_or_expr()?;
                        self.expect_token(Token::RParen)?;
                        q
                    } else {
                        self.parse_unary_expr()?
                    };
                    return Ok(SearchQuery::Child(Box::new(sub_query)));
                }

                "play" => {
                    self.advance();
                    // Check if followed by legal or move keywords
                    let is_legal = if matches!(self.peek(), Some(Token::Ident(s)) if s.eq_ignore_ascii_case("legal"))
                    {
                        self.advance();
                        true
                    } else if matches!(self.peek(), Some(Token::Ident(s)) if s.eq_ignore_ascii_case("move"))
                    {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(s2)) if s2.eq_ignore_ascii_case("legal"))
                        {
                            self.advance();
                            true
                        } else {
                            false
                        }
                    } else {
                        // Default for `play promote Q { ... }` or `play e4 { ... }` is legal move
                        true
                    };

                    // If directly followed by `{`, `(`, `->`, or `leads_to`, it means `play legal { ... }` or `play { ... }`
                    let (move_pat, has_direct_block) =
                        if matches!(self.peek(), Some(Token::LBrace) | Some(Token::LParen)) {
                            (
                                super::query::MovePattern {
                                    is_legal,
                                    ..Default::default()
                                },
                                true,
                            )
                        } else {
                            let q = self.parse_move_filter(is_legal)?;
                            match q {
                                SearchQuery::Play {
                                    move_pattern,
                                    outcome_query,
                                } => {
                                    return Ok(SearchQuery::Play {
                                        move_pattern,
                                        outcome_query,
                                    });
                                }
                                SearchQuery::Move(m) => (m, false),
                                _ => (
                                    super::query::MovePattern {
                                        is_legal,
                                        ..Default::default()
                                    },
                                    false,
                                ),
                            }
                        };

                    let outcome_query = if has_direct_block
                        || matches!(self.peek(), Some(Token::LBrace) | Some(Token::LParen))
                    {
                        if let Some(Token::LBrace) = self.peek() {
                            self.advance();
                            let q = self.parse_or_expr()?;
                            self.expect_token(Token::RBrace)?;
                            q
                        } else if let Some(Token::LParen) = self.peek() {
                            self.advance();
                            let q = self.parse_or_expr()?;
                            self.expect_token(Token::RParen)?;
                            q
                        } else {
                            self.parse_unary_expr()?
                        }
                    } else if matches!(self.peek(), Some(Token::Ident(s)) if s.eq_ignore_ascii_case("leads_to") || s.eq_ignore_ascii_case("leadsto"))
                        || matches!(self.peek(), Some(Token::ArrowRight))
                    {
                        self.advance();
                        if let Some(Token::LBrace) = self.peek() {
                            self.advance();
                            let q = self.parse_or_expr()?;
                            self.expect_token(Token::RBrace)?;
                            q
                        } else if let Some(Token::LParen) = self.peek() {
                            self.advance();
                            let q = self.parse_or_expr()?;
                            self.expect_token(Token::RParen)?;
                            q
                        } else {
                            self.parse_unary_expr()?
                        }
                    } else {
                        self.parse_unary_expr()?
                    };

                    return Ok(SearchQuery::Play {
                        move_pattern: move_pat,
                        outcome_query: Box::new(outcome_query),
                    });
                }

                "for" => {
                    self.advance();
                    let var_name = self.expect_ident()?;
                    self.match_ident("in");
                    let domain = self.parse_variable_domain()?;
                    self.expect_token(Token::LBrace)?;
                    let sub_query = self.parse_or_expr()?;
                    self.expect_token(Token::RBrace)?;
                    return Ok(SearchQuery::VariableBinding {
                        var_name,
                        domain,
                        query: Box::new(sub_query),
                    });
                }

                "what_if" | "whatif" => {
                    self.advance();
                    return self.parse_what_if_expr();
                }

                _ => {
                    if ident_str.starts_with('$') {
                        if self.peek_nth(1) == Some(&Token::Eq) {
                            let var_name = ident_str.to_string();
                            self.advance(); // consume $var
                            self.advance(); // consume '='
                            let domain = self.parse_variable_domain()?;
                            let rest = self.parse_or_expr()?;
                            return Ok(SearchQuery::VariableBinding {
                                var_name,
                                domain,
                                query: Box::new(rest),
                            });
                        } else if self.peek_nth(1).is_some_and(
                            |t| matches!(t, Token::Ident(s) if s.eq_ignore_ascii_case("on")),
                        ) {
                            let var_name = ident_str.to_string();
                            self.advance(); // consume $var
                            self.advance(); // consume 'on'
                            let squares = self.parse_square_set()?;
                            return Ok(SearchQuery::Position(
                                PositionPattern::VariableSquareFilter { var_name, squares },
                            ));
                        }
                    }
                    if let Some((sq, content)) = helpers::parse_compact_piece_placement(&ident_str)
                    {
                        self.advance();
                        let mut map = std::collections::HashMap::new();
                        map.insert(sq, content);
                        return Ok(SearchQuery::Position(PositionPattern::Squares(map)));
                    }
                    if helpers::parse_piece_specifier(&ident_str).is_some() {
                        if self.has_square_set_operator_ahead() {
                            let saved_pos = self.pos;
                            if let Ok(sq) = self.parse_square_set_query() {
                                return Ok(sq);
                            }
                            self.pos = saved_pos;
                        }
                        return self.parse_piece_on_square();
                    }
                }
            }
        }

        // 5. Number-first comparisons: e.g. `34 >= black_power`, `34 >= power`, `30 <= white_power`
        if let Some(Token::Number(num)) = self.peek() {
            let val = *num as i32;
            self.advance();
            let op = self.parse_comparison_op();
            // Since the number is on the left (e.g. 34 >= black_power),
            // comparing `34 >= black_power` is equivalent to `black_power <= 34`.
            // We invert the operator for the right-hand side target.
            let inverted_op = op.invert();

            if let Some(Token::Ident(s)) = self.peek() {
                let key = s.to_lowercase();
                match key.as_str() {
                    "white_power" | "whitepower" => {
                        self.advance();
                        return Ok(SearchQuery::Power(
                            super::query::PowerPredicate::WhitePower {
                                op: inverted_op,
                                value: val,
                            },
                        ));
                    }
                    "black_power" | "blackpower" => {
                        self.advance();
                        return Ok(SearchQuery::Power(
                            super::query::PowerPredicate::BlackPower {
                                op: inverted_op,
                                value: val,
                            },
                        ));
                    }
                    "power" | "total_power" | "totalpower" => {
                        self.advance();
                        return Ok(SearchQuery::Power(
                            super::query::PowerPredicate::TotalPower {
                                op: inverted_op,
                                value: val,
                            },
                        ));
                    }
                    "power_diff" | "power_difference" | "powerdiff" => {
                        self.advance();
                        return Ok(SearchQuery::Power(
                            super::query::PowerPredicate::PowerDifference {
                                op: inverted_op,
                                value: val,
                                absolute: true,
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }

        let saved_pos = self.pos;
        if let Ok(sq) = self.parse_square_set_query() {
            return Ok(sq);
        }
        self.pos = saved_pos;

        Err(ParseError::new(
            format!("Unrecognized query token at pos {}", pos),
            pos,
        ))
    }

    /// Parse a `what_if(...) { ... }` or `what_if [...] { ... }` hypothetical sandbox query
    pub(crate) fn parse_what_if_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        let mut mutations = Vec::new();

        if let Some(Token::LParen) = self.peek() {
            self.advance();
            while let Some(tok) = self.peek() {
                if let Token::RParen = tok {
                    self.advance();
                    break;
                }
                if let Token::Comma = tok {
                    self.advance();
                    continue;
                }
                mutations.push(self.parse_single_board_mutation()?);
            }
        } else if let Some(Token::LBracket) = self.peek() {
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
                mutations.push(self.parse_single_board_mutation()?);
            }
        } else {
            mutations.push(self.parse_single_board_mutation()?);
        }

        if mutations.is_empty() {
            return Err(ParseError::new(
                "what_if requires at least one board mutation (e.g. what_if(pass) { ... }, what_if(remove f6) { ... })",
                pos,
            ));
        }

        let sub_query = if let Some(Token::LBrace) = self.peek() {
            self.advance();
            let q = self.parse_or_expr()?;
            self.expect_token(Token::RBrace)?;
            q
        } else if let Some(Token::LParen) = self.peek() {
            self.advance();
            let q = self.parse_or_expr()?;
            self.expect_token(Token::RParen)?;
            q
        } else {
            self.parse_unary_expr()?
        };

        Ok(SearchQuery::WhatIf {
            mutations,
            query: Box::new(sub_query),
        })
    }

    /// Parse a single board mutation within what_if
    pub(crate) fn parse_single_board_mutation(&mut self) -> Result<BoardMutation, ParseError> {
        let pos = self.current_pos();

        // Bracketed move sequence: `[e4 e5 Qh5]`
        if let Some(Token::LBracket) = self.peek() {
            self.advance();
            let mut moves = Vec::new();
            while let Some(tok) = self.peek() {
                if let Token::RBracket = tok {
                    self.advance();
                    break;
                }
                if let Token::Comma = tok {
                    self.advance();
                    continue;
                }
                let move_pos = self.current_pos();
                if let Some(Token::StringLit(s)) = self.peek() {
                    let s_clone = s.clone();
                    self.advance();
                    if let Some(pat) = helpers::parse_path_move_token(&s_clone) {
                        moves.push(pat);
                    } else {
                        moves.push(super::query::MovePattern {
                            san: Some(s_clone),
                            ..Default::default()
                        });
                    }
                } else if let Some(Token::Ident(s)) = self.peek() {
                    let s_clone = s.clone();
                    self.advance();
                    if let Some(pat) = helpers::parse_path_move_token(&s_clone) {
                        moves.push(pat);
                    } else {
                        return Err(ParseError::new(
                            format!("Invalid move token in what_if sequence: {}", s_clone),
                            move_pos,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        format!("Expected move token in what_if sequence, found {:?}", tok),
                        move_pos,
                    ));
                }
            }
            return Ok(BoardMutation::MoveSequence(moves));
        }

        let kw = self.expect_ident()?;
        let kw_low = kw.to_lowercase();
        match kw_low.as_str() {
            "pass" | "null_move" | "nullmove" | "null" => Ok(BoardMutation::Pass),
            "turn" | "wtm" | "btm" => {
                if kw_low == "wtm" {
                    return Ok(BoardMutation::SetTurn(Color::White));
                }
                if kw_low == "btm" {
                    return Ok(BoardMutation::SetTurn(Color::Black));
                }
                let c_str = self.expect_ident()?;
                let color = match c_str.to_lowercase().as_str() {
                    "white" | "w" => Color::White,
                    "black" | "b" => Color::Black,
                    _ => {
                        return Err(ParseError::new(
                            format!("Invalid turn color in what_if mutation: {}", c_str),
                            pos,
                        ))
                    }
                };
                Ok(BoardMutation::SetTurn(color))
            }
            "remove" | "delete" | "without" => {
                if self.match_ident("knight")
                    || self.match_ident("bishop")
                    || self.match_ident("rook")
                    || self.match_ident("queen")
                    || self.match_ident("pawn")
                    || self.match_ident("king")
                    || self.match_ident("piece")
                {
                    let _ =
                        self.match_ident("on") || self.match_ident("at") || self.match_ident("in");
                }
                let sqs = self.parse_square_set()?;
                Ok(BoardMutation::RemoveSquares(sqs))
            }
            "move" | "transfer" => {
                let from_str = self.expect_ident()?;
                let from = Square::from_str(&from_str.to_lowercase()).map_err(|_| {
                    ParseError::new(
                        format!("Invalid from square in what_if transfer: {}", from_str),
                        pos,
                    )
                })?;
                let _ = self.match_ident("to") || self.match_ident("->");
                let to_str = self.expect_ident()?;
                let to = Square::from_str(&to_str.to_lowercase()).map_err(|_| {
                    ParseError::new(
                        format!("Invalid to square in what_if transfer: {}", to_str),
                        pos,
                    )
                })?;
                Ok(BoardMutation::Transfer { from, to })
            }
            "add" | "place" | "insert" => {
                let p_str = self.expect_ident()?;
                let (c_opt, r_opt) = helpers::parse_piece_specifier(&p_str).ok_or_else(|| {
                    ParseError::new(format!("Invalid piece in what_if add: {}", p_str), pos)
                })?;
                let color = c_opt.unwrap_or(Color::White);
                let role = r_opt.unwrap_or(Role::Queen);
                let piece = Piece { color, role };
                let _ = self.match_ident("on") || self.match_ident("at") || self.match_ident("in");
                let sq_str = self.expect_ident()?;
                let square = Square::from_str(&sq_str.to_lowercase()).map_err(|_| {
                    ParseError::new(format!("Invalid square in what_if add: {}", sq_str), pos)
                })?;
                Ok(BoardMutation::AddPiece { piece, square })
            }
            "swap" => {
                let s1_str = self.expect_ident()?;
                let sq1 = Square::from_str(&s1_str.to_lowercase()).map_err(|_| {
                    ParseError::new(
                        format!("Invalid first square in what_if swap: {}", s1_str),
                        pos,
                    )
                })?;
                if let Some(Token::Comma) = self.peek() {
                    self.advance();
                }
                let s2_str = self.expect_ident()?;
                let sq2 = Square::from_str(&s2_str.to_lowercase()).map_err(|_| {
                    ParseError::new(
                        format!("Invalid second square in what_if swap: {}", s2_str),
                        pos,
                    )
                })?;
                Ok(BoardMutation::SwapSquares { sq1, sq2 })
            }
            "swap_color" | "swapcolor" | "invert_color" | "invertcolor" => {
                let sq_str = self.expect_ident()?;
                let sq = Square::from_str(&sq_str.to_lowercase()).map_err(|_| {
                    ParseError::new(
                        format!("Invalid square in what_if swap_color: {}", sq_str),
                        pos,
                    )
                })?;
                Ok(BoardMutation::SwapColor(sq))
            }
            other => {
                if let Some(pat) = helpers::parse_path_move_token(other) {
                    Ok(BoardMutation::MoveSequence(vec![pat]))
                } else {
                    Err(ParseError::new(
                        format!(
                            "Unknown what_if mutation: '{}'. Supported: pass, turn <color>, remove <squares>, move <from> to <to>, add <piece> on <square>, swap <sq1> <sq2>, swap_color <sq>",
                            other
                        ),
                        pos,
                    ))
                }
            }
        }
    }
}
