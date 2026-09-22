#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::type_complexity,
    clippy::manual_strip
)]

use shakmaty::{Color, Piece, Square};
use std::str::FromStr;

use super::helpers::{expand_square_specifier, parse_piece_specifier};
use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{
    ComparisonOp, CqlLinePattern, CqlPathConstituent, CqlPathPattern, LineDirection, MovePattern,
    PathPattern, PathStep, PositionPattern, SearchQuery, SquareContent,
};

fn parse_token_repetition(s: &str) -> (&str, usize, Option<usize>) {
    if let Some(open_idx) = s.rfind('{') {
        if s.ends_with('}') {
            let inner = &s[open_idx + 1..s.len() - 1];
            let base = &s[..open_idx];
            if let Some((min_s, max_s)) = inner.split_once(',') {
                let min = min_s.trim().parse::<usize>().unwrap_or(0);
                let max = if max_s.trim().is_empty() {
                    None
                } else {
                    max_s.trim().parse::<usize>().ok()
                };
                return (base, min, max);
            } else if let Ok(n) = inner.trim().parse::<usize>() {
                return (base, n, Some(n));
            }
        }
    }
    (s, 1, Some(1))
}

fn validate_move_pattern(pat: &MovePattern, pos: usize) -> Result<(), ParseError> {
    // 1. Validate same-color capture: attacker cannot capture own piece color
    let attacker_color = pat.color.or_else(|| {
        pat.from_pieces.as_ref().and_then(|pcs| {
            for pc in pcs {
                match pc {
                    SquareContent::Piece(p) => return Some(p.color),
                    SquareContent::Color(c) => return Some(*c),
                    _ => {}
                }
            }
            None
        })
    });

    if let Some(att_col) = attacker_color {
        let is_capture = pat.is_capture.unwrap_or(false);
        if let Some(ref to_pcs) = pat.to_pieces {
            for target in to_pcs {
                match target {
                    SquareContent::Piece(p) if p.color == att_col => {
                        return Err(ParseError::new(
                            format!(
                                "Impossible move: {:?} cannot capture {:?} of the same color",
                                att_col, p.role
                            ),
                            pos,
                        ).with_help("A piece cannot capture or move onto a square occupied by a friendly piece."));
                    }
                    SquareContent::Color(c) if *c == att_col && is_capture => {
                        return Err(ParseError::new(
                            format!(
                                "Impossible move: {:?} cannot capture a friendly {:?} piece",
                                att_col, c
                            ),
                            pos,
                        ).with_help("A piece cannot capture friendly pieces. Check piece casing (uppercase for White, lowercase for Black)."));
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn validate_single_color_step(
    pat: &MovePattern,
    single_color: Option<Option<Color>>,
    token_str: &str,
    pos: usize,
) -> Result<(), ParseError> {
    if let Some(Some(required_color)) = single_color {
        let pat_color = pat.color.or_else(|| {
            pat.from_pieces.as_ref().and_then(|pcs| {
                for pc in pcs {
                    match pc {
                        SquareContent::Piece(p) => return Some(p.color),
                        SquareContent::Color(c) => return Some(*c),
                        _ => {}
                    }
                }
                None
            })
        });

        if let Some(col) = pat_color {
            if col != required_color {
                let req_str = match required_color {
                    Color::White => "white",
                    Color::Black => "black",
                };
                let col_str = match col {
                    Color::White => "White",
                    Color::Black => "Black",
                };
                return Err(ParseError::new(
                    format!(
                        "Color mismatch in path {}: move '{}' is for {} pieces",
                        req_str, token_str, col_str
                    ),
                    pos,
                ).with_help(format!(
                    "In 'path {}', all moves must belong to {}. Remember uppercase pieces (e.g. B, N, R, Q, K, P) are White and lowercase (e.g. b, n, r, q, k, p) are Black.",
                    req_str, req_str
                )));
            }
        }
    }
    Ok(())
}

impl<'a> QueryParser<'a> {
    pub(crate) fn parse_path_expr(
        &mut self,
        consecutive: bool,
        single_color: Option<Option<Color>>,
    ) -> Result<SearchQuery, ParseError> {
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
                } else {
                    let (tok_str, repeat_min, repeat_max) = parse_token_repetition(token);
                    if let Some(pat) = super::helpers::parse_path_move_token(tok_str) {
                        validate_single_color_step(&pat, single_color, tok_str, pos)?;
                        validate_move_pattern(&pat, pos)?;
                        moves.push(pat.clone());
                        steps.push(PathStep::Move {
                            pattern: pat,
                            repeat_min,
                            repeat_max,
                        });
                    }
                }
            }
            return Ok(SearchQuery::Path(PathPattern {
                steps,
                moves,
                consecutive,
                single_color,
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
                    let (tok_str, repeat_min, repeat_max) = parse_token_repetition(&s_clone);
                    if let Some(pat) = super::helpers::parse_path_move_token(tok_str) {
                        validate_single_color_step(&pat, single_color, tok_str, pos)?;
                        validate_move_pattern(&pat, pos)?;
                        moves.push(pat.clone());
                        steps.push(PathStep::Move {
                            pattern: pat,
                            repeat_min,
                            repeat_max,
                        });
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

                    if (full_token.len() == 1
                        || (full_token.starts_with('[') && full_token.ends_with(']')))
                        && matches!(self.peek(), Some(Token::Ident(s_sep)) if s_sep == "--" || s_sep == "->")
                    {
                        if let Some((_, Token::Ident(s_target))) = self.tokens.get(self.pos + 1) {
                            if super::helpers::parse_direction_ident(s_target).is_some() {
                                full_token.push_str("--");
                                self.advance(); // consume '--'
                                full_token.push_str(s_target);
                                self.advance(); // consume dir ident
                                if let Some(Token::Number(n_target)) = self.peek() {
                                    full_token.push(' ');
                                    full_token.push_str(&n_target.to_string());
                                    self.advance();
                                }
                            }
                        }
                    }

                    if full_token.contains("--") || full_token.contains("->") {
                        if let Some(Token::Number(n)) = self.peek() {
                            full_token.push(' ');
                            full_token.push_str(&n.to_string());
                            self.advance();
                        }
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

                    let mut repeat_min = 1;
                    let mut repeat_max = Some(1);

                    // Check if followed by repetition quantifier `{min,max}` or `{count}`
                    if let Some(Token::LBrace) = self.peek() {
                        self.advance();
                        let mut min = 0;
                        let mut max = None;
                        if let Some(Token::Number(n)) = self.peek() {
                            min = *n as usize;
                            max = Some(min);
                            self.advance();
                        }
                        if let Some(Token::Comma) = self.peek() {
                            self.advance();
                            if let Some(Token::Number(n2)) = self.peek() {
                                max = Some(*n2 as usize);
                                self.advance();
                            } else {
                                max = None;
                            }
                        }
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        repeat_min = min;
                        repeat_max = max;
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
                        validate_single_color_step(&pat, single_color, &full_token, pos)?;
                        validate_move_pattern(&pat, pos)?;
                        moves.push(pat.clone());
                        steps.push(PathStep::Move {
                            pattern: pat,
                            repeat_min,
                            repeat_max,
                        });
                    }
                    continue;
                }

                self.advance();
            }

            return Ok(SearchQuery::Path(PathPattern {
                steps,
                moves,
                consecutive,
                single_color,
                max_gap_plies: if consecutive { None } else { Some(30) },
                start_ply_range: None,
            }));
        }

        Err(ParseError::new(
            "Expected '[' or string after line/path".to_string(),
            pos,
        ))
    }

    /// Parse full CQL 6.2 `cql_path` / `cqlpath` / `turnstile` / `sequence` expression
    pub(crate) fn parse_cql_path_expr(
        &mut self,
        single_color: Option<Option<Color>>,
    ) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        let mut constituents = Vec::new();

        let closing_token = if let Some(Token::LBrace) = self.peek() {
            self.advance();
            Token::RBrace
        } else if let Some(Token::LBracket) = self.peek() {
            self.advance();
            Token::RBracket
        } else if let Some(Token::LParen) = self.peek() {
            self.advance();
            Token::RParen
        } else {
            return Err(ParseError::new(
                "Expected '{', '[', or '(' after cql_path".to_string(),
                pos,
            ));
        };

        while let Some(tok) = self.peek() {
            if std::mem::discriminant(tok) == std::mem::discriminant(&closing_token) {
                self.advance();
                break;
            }

            if matches!(tok, Token::Comma) {
                self.advance();
                continue;
            }

            // 1. Nested general query filter block: `{ attacks(N, q) }`
            if matches!(tok, Token::LBrace) {
                self.advance();
                let sub_query = self.parse_or_expr()?;
                self.expect_token(Token::RBrace)?;
                let mut constituent = CqlPathConstituent::Filter(Box::new(sub_query));
                if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                    constituent = CqlPathConstituent::Repetition {
                        constituent: Box::new(constituent),
                        min: rep_min,
                        max: rep_max,
                    };
                }
                constituents.push(constituent);
                continue;
            }

            // 2. Parenthesized chain constituent: `( Bxh7+ kxh7 )*`
            if matches!(tok, Token::LParen) {
                self.advance();
                let mut chain_items = Vec::new();
                while let Some(chain_tok) = self.peek() {
                    if matches!(chain_tok, Token::RParen) {
                        self.advance();
                        break;
                    }
                    if matches!(chain_tok, Token::Comma) {
                        self.advance();
                        continue;
                    }
                    let item = self.parse_cql_path_single_constituent(single_color)?;
                    chain_items.push(item);
                }

                let mut chain_const = CqlPathConstituent::Chain(chain_items);
                if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                    chain_const = CqlPathConstituent::Repetition {
                        constituent: Box::new(chain_const),
                        min: rep_min,
                        max: rep_max,
                    };
                }
                constituents.push(chain_const);
                continue;
            }

            // 3. Single constituent (Move or keyword filter)
            let item = self.parse_cql_path_single_constituent(single_color)?;
            constituents.push(item);
        }

        Ok(SearchQuery::CqlPath(CqlPathPattern {
            constituents,
            single_color,
            start_ply_range: None,
        }))
    }

    fn parse_cql_path_single_constituent(
        &mut self,
        single_color: Option<Option<Color>>,
    ) -> Result<CqlPathConstituent, ParseError> {
        let pos = self.current_pos();

        // 1. Check for nested filter block `{ ... }`
        if self.peek() == Some(&Token::LBrace) {
            self.advance();
            let filter_query = self.parse_or_expr()?;
            match self.peek() {
                Some(Token::RBrace) => {
                    self.advance();
                }
                tok => {
                    return Err(ParseError::new(
                        format!("Expected '}}' closing filter block, found {:?}", tok),
                        pos,
                    ));
                }
            }
            let mut constituent = CqlPathConstituent::Filter(Box::new(filter_query));
            if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                constituent = CqlPathConstituent::Repetition {
                    constituent: Box::new(constituent),
                    min: rep_min,
                    max: rep_max,
                };
            }
            return Ok(constituent);
        }

        // 2. Check for standalone filter keywords
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            match id_low.as_str() {
                "not" => {
                    self.advance();
                    if let Some(Token::Ident(ref next_id)) = self.peek() {
                        let next_low = next_id.to_lowercase();
                        if next_low == "check" || next_low == "is_check" {
                            self.advance();
                            return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Not(
                                Box::new(SearchQuery::Position(PositionPattern::BoardState {
                                    is_check: Some(true),
                                    is_checkmate: None,
                                    is_stalemate: None,
                                })),
                            ))));
                        }
                    }
                    let sub = self.parse_primary_expr()?;
                    return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Not(
                        Box::new(sub),
                    ))));
                }
                "check" | "is_check" => {
                    self.advance();
                    return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Position(
                        PositionPattern::BoardState {
                            is_check: Some(true),
                            is_checkmate: None,
                            is_stalemate: None,
                        },
                    ))));
                }
                "mate" | "checkmate" | "is_mate" | "is_checkmate" => {
                    self.advance();
                    return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Position(
                        PositionPattern::BoardState {
                            is_check: None,
                            is_checkmate: Some(true),
                            is_stalemate: None,
                        },
                    ))));
                }
                "stalemate" => {
                    self.advance();
                    return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Position(
                        PositionPattern::BoardState {
                            is_check: None,
                            is_checkmate: None,
                            is_stalemate: Some(true),
                        },
                    ))));
                }
                "wtm" => {
                    self.advance();
                    return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Position(
                        PositionPattern::Turn(Color::White),
                    ))));
                }
                "btm" => {
                    self.advance();
                    return Ok(CqlPathConstituent::Filter(Box::new(SearchQuery::Position(
                        PositionPattern::Turn(Color::Black),
                    ))));
                }
                _ => {}
            }
        }

        // 3. Otherwise parse as a Move token
        let tok = self.peek().cloned().ok_or_else(|| {
            ParseError::new("Unexpected end of tokens in cql_path".to_string(), pos)
        })?;

        let mut raw_str = match tok {
            Token::Ident(ref s) => {
                self.advance();
                s.clone()
            }
            Token::StringLit(ref s) => {
                self.advance();
                s.clone()
            }
            _ => {
                return Err(ParseError::new(
                    format!("Unexpected token in cql_path: {:?}", tok),
                    pos,
                ));
            }
        };

        // If followed by trailing suffix tokens e.g. `+`, `#`, `=`, `check`
        if let Some(Token::Ident(ref s_next)) = self.peek() {
            if s_next.starts_with('+')
                || s_next.starts_with('#')
                || s_next.starts_with('=')
                || s_next.starts_with('!')
                || s_next.starts_with('?')
            {
                raw_str.push_str(s_next);
                self.advance();
            }
        }

        let mut pat = super::helpers::parse_path_move_token(&raw_str).ok_or_else(|| {
            ParseError::new(
                format!("Invalid move pattern in cql_path: {}", raw_str),
                pos,
            )
        })?;

        // Check for attached keywords: e.g. `Bxh7 check`, `e4 not check`, `Rd8 mate`
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            if id_low == "check" || id_low == "is_check" {
                self.advance();
                pat.is_check = Some(true);
            } else if id_low == "not" {
                if let Some(Token::Ident(ref next_id)) = self.peek_nth(1) {
                    if next_id.eq_ignore_ascii_case("check") {
                        self.advance(); // consume 'not'
                        self.advance(); // consume 'check'
                        pat.is_check = Some(false);
                    }
                }
            } else if id_low == "mate" || id_low == "checkmate" {
                self.advance();
                pat.is_checkmate = Some(true);
            }
        }

        let is_explicit_color = raw_str.starts_with('w')
            || raw_str.starts_with('b')
            || raw_str.starts_with("white")
            || raw_str.starts_with("black")
            || raw_str.starts_with('[');

        if single_color.is_none() && !is_explicit_color {
            pat.color = None;
            pat.from_pieces = None;
        }

        validate_single_color_step(&pat, single_color, &raw_str, pos)?;
        validate_move_pattern(&pat, pos)?;

        let mut constituent = CqlPathConstituent::Move(pat);
        if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
            constituent = CqlPathConstituent::Repetition {
                constituent: Box::new(constituent),
                min: rep_min,
                max: rep_max,
            };
        }

        Ok(constituent)
    }

    fn parse_cql_path_repetition_quantifier(&mut self) -> Option<(usize, Option<usize>)> {
        if let Some(tok) = self.peek() {
            match tok {
                Token::Ident(ref s) => {
                    if s == "*" {
                        self.advance();
                        return Some((0, None));
                    }
                    if s == "+" {
                        self.advance();
                        return Some((1, None));
                    }
                    if s == "?" {
                        self.advance();
                        return Some((0, Some(1)));
                    }
                }
                Token::LBrace => {
                    if let Some(Token::Number(n)) = self.peek_nth(1) {
                        let min_val = *n as usize;
                        self.advance(); // consume '{'
                        self.advance(); // consume Number
                        let mut max = Some(min_val);
                        if let Some(Token::Comma) = self.peek() {
                            self.advance();
                            if let Some(Token::Number(n2)) = self.peek() {
                                max = Some(*n2 as usize);
                                self.advance();
                            } else {
                                max = None;
                            }
                        }
                        if let Some(Token::RBrace) = self.peek() {
                            self.advance();
                        }
                        return Some((min_val, max));
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Dispatch between legacy SCID `line [e4 e5]` move path and modern CQLi `line --> ...` filter
    pub(crate) fn parse_cql_line_or_legacy_path(
        &mut self,
        single_color: Option<Option<Color>>,
    ) -> Result<SearchQuery, ParseError> {
        // Check if legacy bracket path: e.g. `line [e4 e5 Nf3]` vs `line [5 100] --> ...` vs `line --> ...`
        if let Some(Token::LBracket) = self.peek() {
            let is_numeric_range = matches!(self.peek_nth(1), Some(Token::Number(_)));

            // Also check if there's an arrow anywhere ahead
            let mut has_arrow = false;
            for i in 1..self.tokens.len().saturating_sub(self.pos) {
                match self.peek_nth(i) {
                    Some(Token::ArrowRight) | Some(Token::ArrowLeft) => {
                        has_arrow = true;
                        break;
                    }
                    _ => {}
                }
            }

            if !is_numeric_range && !has_arrow {
                // Legacy SCID line [e4 e5 ...] syntax
                return self.parse_path_expr(true, single_color);
            }
        }

        // If string literal without arrow: e.g. `line "e4 e5"`
        if let Some(Token::StringLit(_)) = self.peek() {
            let mut has_arrow = false;
            for i in 1..self.tokens.len().saturating_sub(self.pos) {
                match self.peek_nth(i) {
                    Some(Token::ArrowRight) | Some(Token::ArrowLeft) => {
                        has_arrow = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !has_arrow {
                return self.parse_path_expr(true, single_color);
            }
        }

        self.parse_cql_line_expr(single_color)
    }

    /// Parse modern CQLi `line [range_min range_max] [parameters] {--> | <--} constituent1 ...` expression
    pub(crate) fn parse_cql_line_expr(
        &mut self,
        initial_single_color: Option<Option<Color>>,
    ) -> Result<SearchQuery, ParseError> {
        let mut min_length = None;
        let mut max_length = None;
        let mut single_color = initial_single_color;
        let mut first_match = false;
        let mut last_position = false;
        let mut nest_ban = false;
        let mut primary_only = true;

        // Parse optional modifiers, range, singlecolor before the first arrow
        loop {
            if let Some(Token::ArrowRight) | Some(Token::ArrowLeft) = self.peek() {
                break;
            }

            // 1. Bracketed range: `[min max]` or `[count]` or `[min, max]`
            if let Some(Token::LBracket) = self.peek() {
                if let Some(Token::Number(n1)) = self.peek_nth(1) {
                    let min_v = *n1 as usize;
                    self.advance(); // consume '['
                    self.advance(); // consume Number
                    let mut max_v = Some(min_v);
                    if let Some(Token::Comma) = self.peek() {
                        self.advance();
                    }
                    if let Some(Token::Number(n2)) = self.peek() {
                        max_v = Some(*n2 as usize);
                        self.advance();
                    }
                    if let Some(Token::RBracket) = self.peek() {
                        self.advance();
                    }
                    min_length = Some(min_v);
                    max_length = max_v;
                    continue;
                }
            }

            // 2. Bare numbers: `5 100` or `5`
            if let Some(Token::Number(n1)) = self.peek() {
                let min_v = *n1 as usize;
                self.advance();
                let mut max_v = None;
                if let Some(Token::Number(n2)) = self.peek() {
                    max_v = Some(*n2 as usize);
                    self.advance();
                }
                min_length = Some(min_v);
                max_length = max_v;
                continue;
            }

            // 3. Identifiers
            if let Some(Token::Ident(ref s)) = self.peek() {
                let s_low = s.to_lowercase();
                match s_low.as_str() {
                    "firstmatch" | "first_match" => {
                        self.advance();
                        first_match = true;
                        continue;
                    }
                    "lastposition" | "last_position" => {
                        self.advance();
                        last_position = true;
                        continue;
                    }
                    "nestban" | "nest_ban" => {
                        self.advance();
                        nest_ban = true;
                        continue;
                    }
                    "singlecolor" | "single_color" => {
                        self.advance();
                        single_color = Some(None);
                        continue;
                    }
                    "white" => {
                        self.advance();
                        single_color = Some(Some(Color::White));
                        continue;
                    }
                    "black" => {
                        self.advance();
                        single_color = Some(Some(Color::Black));
                        continue;
                    }
                    "primary" => {
                        self.advance();
                        primary_only = true;
                        continue;
                    }
                    "secondary" => {
                        self.advance();
                        primary_only = false;
                        continue;
                    }
                    "quiet" | "nolinearize" | "nonatomic" => {
                        self.advance();
                        continue;
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }

        // Must encounter first arrow
        let first_arrow_pos = self.current_pos();
        let direction = match self.peek() {
            Some(Token::ArrowRight) => {
                self.advance();
                LineDirection::Forward
            }
            Some(Token::ArrowLeft) => {
                self.advance();
                LineDirection::Backward
            }
            Some(tok) => {
                return Err(ParseError::new(
                    format!("Expected '-->' or '<--' in line filter, found {:?}", tok),
                    first_arrow_pos,
                ).with_help("CQLi line filters connect constituents with directional arrows, e.g. 'line --> check+' or 'line <-- check*'."));
            }
            None => {
                return Err(ParseError::new(
                    "Expected '-->' or '<--' in line filter, found EOF".to_string(),
                    first_arrow_pos,
                ).with_help("CQLi line filters connect constituents with directional arrows, e.g. 'line --> check+' or 'line <-- check*'."));
            }
        };

        let mut constituents = Vec::new();

        loop {
            let item = self.parse_cql_line_constituent(direction, single_color)?;
            constituents.push(item);

            // Check if followed by next arrow
            match self.peek() {
                Some(Token::ArrowRight) => {
                    let arrow_pos = self.current_pos();
                    if direction != LineDirection::Forward {
                        return Err(ParseError::new(
                            "Mixing '-->' and '<--' within the same line filter is invalid".to_string(),
                            arrow_pos,
                        ).with_help("All constituents in a line filter must use the same directional arrow ('-->' forward or '<--' backward)."));
                    }
                    self.advance();
                }
                Some(Token::ArrowLeft) => {
                    let arrow_pos = self.current_pos();
                    if direction != LineDirection::Backward {
                        return Err(ParseError::new(
                            "Mixing '-->' and '<--' within the same line filter is invalid".to_string(),
                            arrow_pos,
                        ).with_help("All constituents in a line filter must use the same directional arrow ('-->' forward or '<--' backward)."));
                    }
                    self.advance();
                }
                _ => break,
            }
        }

        Ok(SearchQuery::CqlLine(CqlLinePattern {
            min_length,
            max_length,
            direction,
            single_color,
            first_match,
            last_position,
            nest_ban,
            primary_only,
            start_ply_range: None,
            constituents,
        }))
    }

    fn parse_cql_line_constituent(
        &mut self,
        direction: LineDirection,
        single_color: Option<Option<Color>>,
    ) -> Result<CqlPathConstituent, ParseError> {
        let pos = self.current_pos();

        // 1. Nested filter block `{ ... }`
        if self.peek() == Some(&Token::LBrace) {
            self.advance();
            let filter_query = self.parse_or_expr()?;
            self.expect_token(Token::RBrace)?;
            let mut constituent = CqlPathConstituent::Filter(Box::new(filter_query));
            if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                constituent = CqlPathConstituent::Repetition {
                    constituent: Box::new(constituent),
                    min: rep_min,
                    max: rep_max,
                };
            }
            return Ok(constituent);
        }

        // 2. Parenthesized group `( ... )`
        if self.peek() == Some(&Token::LParen) {
            self.advance();
            let mut chain_items = Vec::new();
            while let Some(tok) = self.peek() {
                if matches!(tok, Token::RParen) {
                    self.advance();
                    break;
                }
                if matches!(tok, Token::Comma) {
                    self.advance();
                    continue;
                }
                // If arrow inside parenthesized group
                if matches!(tok, Token::ArrowRight) {
                    if direction != LineDirection::Forward {
                        return Err(ParseError::new(
                            "Mixing '-->' and '<--' within the same line filter is invalid"
                                .to_string(),
                            self.current_pos(),
                        ));
                    }
                    self.advance();
                    continue;
                }
                if matches!(tok, Token::ArrowLeft) {
                    if direction != LineDirection::Backward {
                        return Err(ParseError::new(
                            "Mixing '-->' and '<--' within the same line filter is invalid"
                                .to_string(),
                            self.current_pos(),
                        ));
                    }
                    self.advance();
                    continue;
                }

                let item = self.parse_cql_line_constituent(direction, single_color)?;
                chain_items.push(item);
            }

            let mut chain_const = CqlPathConstituent::Chain(chain_items);
            if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                chain_const = CqlPathConstituent::Repetition {
                    constituent: Box::new(chain_const),
                    min: rep_min,
                    max: rep_max,
                };
            }
            return Ok(chain_const);
        }

        // 3. Move / Legal / Previous filter keywords
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            if id_low == "move" || id_low == "previous" || id_low == "prev" || id_low == "legal" {
                let is_legal = id_low == "legal";
                let is_prev = id_low == "previous" || id_low == "prev";
                self.advance();
                if is_prev
                    && matches!(self.peek(), Some(Token::Ident(ref s)) if s.eq_ignore_ascii_case("move"))
                {
                    self.advance();
                }
                let mut q = self.parse_move_filter(is_legal)?;
                if is_prev {
                    if let SearchQuery::Move(ref mut m) = q {
                        m.is_previous = true;
                    }
                }
                let mut constituent = CqlPathConstituent::Filter(Box::new(q));
                if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                    constituent = CqlPathConstituent::Repetition {
                        constituent: Box::new(constituent),
                        min: rep_min,
                        max: rep_max,
                    };
                }
                return Ok(constituent);
            }

            // Standalone piece filter name: `Queen`, `Rook`, `Bishop`, `Knight`, `Pawn`, `King`
            let piece_role_opt = match id_low.as_str() {
                "queen" => Some(shakmaty::Role::Queen),
                "rook" => Some(shakmaty::Role::Rook),
                "bishop" => Some(shakmaty::Role::Bishop),
                "knight" => Some(shakmaty::Role::Knight),
                "pawn" => Some(shakmaty::Role::Pawn),
                "king" => Some(shakmaty::Role::King),
                _ => None,
            };
            if let Some(role) = piece_role_opt {
                self.advance();
                let mut constituent = CqlPathConstituent::Filter(Box::new(SearchQuery::Position(
                    PositionPattern::PieceCount {
                        content: SquareContent::Role(role),
                        squares: None,
                        op: ComparisonOp::GreaterThanOrEqual,
                        count: 1,
                    },
                )));
                if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                    constituent = CqlPathConstituent::Repetition {
                        constituent: Box::new(constituent),
                        min: rep_min,
                        max: rep_max,
                    };
                }
                return Ok(constituent);
            }

            // Standalone filter keywords
            match id_low.as_str() {
                "not" => {
                    self.advance();
                    if let Some(Token::Ident(ref next_id)) = self.peek() {
                        let next_low = next_id.to_lowercase();
                        if next_low == "check" || next_low == "is_check" {
                            self.advance();
                            let mut constituent =
                                CqlPathConstituent::Filter(Box::new(SearchQuery::Not(Box::new(
                                    SearchQuery::Position(PositionPattern::BoardState {
                                        is_check: Some(true),
                                        is_checkmate: None,
                                        is_stalemate: None,
                                    }),
                                ))));
                            if let Some((rep_min, rep_max)) =
                                self.parse_cql_path_repetition_quantifier()
                            {
                                constituent = CqlPathConstituent::Repetition {
                                    constituent: Box::new(constituent),
                                    min: rep_min,
                                    max: rep_max,
                                };
                            }
                            return Ok(constituent);
                        }
                    }
                    let sub = self.parse_primary_expr()?;
                    let mut constituent =
                        CqlPathConstituent::Filter(Box::new(SearchQuery::Not(Box::new(sub))));
                    if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                        constituent = CqlPathConstituent::Repetition {
                            constituent: Box::new(constituent),
                            min: rep_min,
                            max: rep_max,
                        };
                    }
                    return Ok(constituent);
                }
                "check" | "is_check" => {
                    self.advance();
                    let mut constituent = CqlPathConstituent::Filter(Box::new(
                        SearchQuery::Position(PositionPattern::BoardState {
                            is_check: Some(true),
                            is_checkmate: None,
                            is_stalemate: None,
                        }),
                    ));
                    if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                        constituent = CqlPathConstituent::Repetition {
                            constituent: Box::new(constituent),
                            min: rep_min,
                            max: rep_max,
                        };
                    }
                    return Ok(constituent);
                }
                "mate" | "checkmate" | "is_mate" | "is_checkmate" => {
                    self.advance();
                    let mut constituent = CqlPathConstituent::Filter(Box::new(
                        SearchQuery::Position(PositionPattern::BoardState {
                            is_check: None,
                            is_checkmate: Some(true),
                            is_stalemate: None,
                        }),
                    ));
                    if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                        constituent = CqlPathConstituent::Repetition {
                            constituent: Box::new(constituent),
                            min: rep_min,
                            max: rep_max,
                        };
                    }
                    return Ok(constituent);
                }
                "stalemate" => {
                    self.advance();
                    let mut constituent = CqlPathConstituent::Filter(Box::new(
                        SearchQuery::Position(PositionPattern::BoardState {
                            is_check: None,
                            is_checkmate: None,
                            is_stalemate: Some(true),
                        }),
                    ));
                    if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                        constituent = CqlPathConstituent::Repetition {
                            constituent: Box::new(constituent),
                            min: rep_min,
                            max: rep_max,
                        };
                    }
                    return Ok(constituent);
                }
                "wtm" => {
                    self.advance();
                    let mut constituent = CqlPathConstituent::Filter(Box::new(
                        SearchQuery::Position(PositionPattern::Turn(Color::White)),
                    ));
                    if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                        constituent = CqlPathConstituent::Repetition {
                            constituent: Box::new(constituent),
                            min: rep_min,
                            max: rep_max,
                        };
                    }
                    return Ok(constituent);
                }
                "btm" => {
                    self.advance();
                    let mut constituent = CqlPathConstituent::Filter(Box::new(
                        SearchQuery::Position(PositionPattern::Turn(Color::Black)),
                    ));
                    if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
                        constituent = CqlPathConstituent::Repetition {
                            constituent: Box::new(constituent),
                            min: rep_min,
                            max: rep_max,
                        };
                    }
                    return Ok(constituent);
                }
                _ => {
                    // Check for attached repetition on keyword: e.g. `check+`, `check*`, `check?`, `mate+`, `check{3}`
                    let (base, rep_min, rep_max) = parse_token_repetition(&id_low);
                    let attached_rep = if base.ends_with('+') {
                        Some((&base[..base.len() - 1], 1, None))
                    } else if base.ends_with('*') {
                        Some((&base[..base.len() - 1], 0, None))
                    } else if base.ends_with('?') {
                        Some((&base[..base.len() - 1], 0, Some(1)))
                    } else if rep_min != 1 || rep_max != Some(1) {
                        Some((base, rep_min, rep_max))
                    } else {
                        None
                    };

                    if let Some((clean_base, r_min, r_max)) = attached_rep {
                        let filter_q_opt = match clean_base {
                            "check" | "is_check" => {
                                Some(SearchQuery::Position(PositionPattern::BoardState {
                                    is_check: Some(true),
                                    is_checkmate: None,
                                    is_stalemate: None,
                                }))
                            }
                            "mate" | "checkmate" | "is_mate" | "is_checkmate" => {
                                Some(SearchQuery::Position(PositionPattern::BoardState {
                                    is_check: None,
                                    is_checkmate: Some(true),
                                    is_stalemate: None,
                                }))
                            }
                            "stalemate" => {
                                Some(SearchQuery::Position(PositionPattern::BoardState {
                                    is_check: None,
                                    is_checkmate: None,
                                    is_stalemate: Some(true),
                                }))
                            }
                            "wtm" => {
                                Some(SearchQuery::Position(PositionPattern::Turn(Color::White)))
                            }
                            "btm" => {
                                Some(SearchQuery::Position(PositionPattern::Turn(Color::Black)))
                            }
                            "queen" => Some(SearchQuery::Position(PositionPattern::PieceCount {
                                content: SquareContent::Role(shakmaty::Role::Queen),
                                squares: None,
                                op: ComparisonOp::GreaterThanOrEqual,
                                count: 1,
                            })),
                            "rook" => Some(SearchQuery::Position(PositionPattern::PieceCount {
                                content: SquareContent::Role(shakmaty::Role::Rook),
                                squares: None,
                                op: ComparisonOp::GreaterThanOrEqual,
                                count: 1,
                            })),
                            "bishop" => Some(SearchQuery::Position(PositionPattern::PieceCount {
                                content: SquareContent::Role(shakmaty::Role::Bishop),
                                squares: None,
                                op: ComparisonOp::GreaterThanOrEqual,
                                count: 1,
                            })),
                            "knight" => Some(SearchQuery::Position(PositionPattern::PieceCount {
                                content: SquareContent::Role(shakmaty::Role::Knight),
                                squares: None,
                                op: ComparisonOp::GreaterThanOrEqual,
                                count: 1,
                            })),
                            "pawn" => Some(SearchQuery::Position(PositionPattern::PieceCount {
                                content: SquareContent::Role(shakmaty::Role::Pawn),
                                squares: None,
                                op: ComparisonOp::GreaterThanOrEqual,
                                count: 1,
                            })),
                            _ => None,
                        };

                        if let Some(fq) = filter_q_opt {
                            self.advance();
                            return Ok(CqlPathConstituent::Repetition {
                                constituent: Box::new(CqlPathConstituent::Filter(Box::new(fq))),
                                min: r_min,
                                max: r_max,
                            });
                        }
                    }
                }
            }
        }

        // 4. Otherwise parse as a Move token
        let tok = self.peek().cloned().ok_or_else(|| {
            ParseError::new("Unexpected end of tokens in line filter".to_string(), pos)
        })?;

        let mut raw_str = match tok {
            Token::Ident(ref s) => {
                self.advance();
                s.clone()
            }
            Token::StringLit(ref s) => {
                self.advance();
                s.clone()
            }
            _ => {
                return Err(ParseError::new(
                    format!("Unexpected token in line filter: {:?}", tok),
                    pos,
                ));
            }
        };

        // If followed by trailing suffix tokens e.g. `+`, `#`, `=`, `check`
        if let Some(Token::Ident(ref s_next)) = self.peek() {
            if s_next.starts_with('+')
                || s_next.starts_with('#')
                || s_next.starts_with('=')
                || s_next.starts_with('!')
                || s_next.starts_with('?')
            {
                raw_str.push_str(s_next);
                self.advance();
            }
        }

        let (clean_tok, attached_min, attached_max) = parse_token_repetition(&raw_str);
        let mut pat = super::helpers::parse_path_move_token(clean_tok).ok_or_else(|| {
            ParseError::new(
                format!("Invalid move pattern in line filter: {}", raw_str),
                pos,
            )
        })?;

        // Check for attached keywords: e.g. `Bxh7 check`, `e4 not check`, `Rd8 mate`
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            if id_low == "check" || id_low == "is_check" {
                self.advance();
                pat.is_check = Some(true);
            } else if id_low == "not" {
                if let Some(Token::Ident(ref next_id)) = self.peek_nth(1) {
                    if next_id.eq_ignore_ascii_case("check") {
                        self.advance(); // consume 'not'
                        self.advance(); // consume 'check'
                        pat.is_check = Some(false);
                    }
                }
            } else if id_low == "mate" || id_low == "checkmate" {
                self.advance();
                pat.is_checkmate = Some(true);
            }
        }

        let is_explicit_color = raw_str.starts_with('w')
            || raw_str.starts_with('b')
            || raw_str.starts_with("white")
            || raw_str.starts_with("black")
            || raw_str.starts_with('[');

        if single_color.is_none() && !is_explicit_color {
            pat.color = None;
            pat.from_pieces = None;
        }

        validate_single_color_step(&pat, single_color, clean_tok, pos)?;
        validate_move_pattern(&pat, pos)?;

        let mut constituent = CqlPathConstituent::Move(pat);
        if attached_min != 1 || attached_max != Some(1) {
            constituent = CqlPathConstituent::Repetition {
                constituent: Box::new(constituent),
                min: attached_min,
                max: attached_max,
            };
        } else if let Some((rep_min, rep_max)) = self.parse_cql_path_repetition_quantifier() {
            constituent = CqlPathConstituent::Repetition {
                constituent: Box::new(constituent),
                min: rep_min,
                max: rep_max,
            };
        }

        Ok(constituent)
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
                            if let Some(Token::Ident(ref to_id)) = self.peek() {
                                if let Some(dir) = super::helpers::parse_direction_ident(to_id) {
                                    self.advance();
                                    let mut min_dist = 1;
                                    let mut max_dist = None;
                                    if let Some(Token::Number(n)) = self.peek() {
                                        min_dist = *n as usize;
                                        self.advance();
                                        if let Some(Token::Number(n2)) = self.peek() {
                                            max_dist = Some(*n2 as usize);
                                            self.advance();
                                        } else {
                                            max_dist = Some(min_dist);
                                        }
                                    }
                                    pattern.direction = Some((dir, min_dist, max_dist));
                                    continue;
                                }
                            }
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
                            if matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                                || matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in" || in_tok.to_lowercase() == "of")
                            {
                                self.advance();
                            }

                            if let Some(Token::LBracket) = self.peek() {
                                let (_sqs_opt, pcs_opt) = self.parse_square_or_piece_set()?;
                                pattern.captured_pieces = pcs_opt;
                            } else if let Some(Token::Ident(ref s)) = self.peek() {
                                let s_low = s.to_lowercase();
                                let is_keyword = matches!(
                                    s_low.as_str(),
                                    "legal"
                                        | "from"
                                        | "to"
                                        | "promote"
                                        | "promotion"
                                        | "promotes"
                                        | "check"
                                        | "is_check"
                                        | "mate"
                                        | "checkmate"
                                        | "is_mate"
                                        | "is_checkmate"
                                        | "prev"
                                        | "previous"
                                        | "castle"
                                        | "castling"
                                        | "o-o"
                                        | "o-o-o"
                                        | "oo"
                                        | "ooo"
                                        | "short"
                                        | "long"
                                        | "en_passant"
                                        | "ep"
                                        | "count"
                                        | "capture"
                                        | "is_capture"
                                ) || super::helpers::parse_direction_ident(&s_low)
                                    .is_some();

                                if !is_keyword {
                                    if s_low == "piece" {
                                        self.advance();
                                    }
                                    if let Some(tok_after) = self.peek() {
                                        if let Token::LBracket | Token::Ident(_) = tok_after {
                                            let (_sqs_opt, pcs_opt) =
                                                self.parse_square_or_piece_set()?;
                                            pattern.captured_pieces = pcs_opt;
                                        }
                                    }
                                }
                            }
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
                                        | "mate"
                                        | "checkmate"
                                        | "is_mate"
                                        | "is_checkmate"
                                        | "from"
                                        | "to"
                                        | "piece"
                                        | "capture"
                                        | "is_capture"
                                        | "legal"
                                        | "leads_to"
                                        | "leadsto"
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
                        "mate" | "checkmate" | "is_mate" | "is_checkmate" => {
                            self.advance();
                            pattern.is_checkmate = Some(true);
                        }
                        "prev" | "previous" => {
                            self.advance();
                            pattern.is_previous = true;
                        }
                        "castle" | "castling" | "o-o" | "o-o-o" | "oo" | "ooo" | "short"
                        | "long" => {
                            self.advance();
                            pattern.is_castle = Some(true);
                        }
                        "en_passant" | "ep" => {
                            self.advance();
                            pattern.is_en_passant = Some(true);
                        }
                        "count" => {
                            self.advance();
                            let op = self.parse_comparison_op();
                            let count = self.expect_number()? as usize;
                            pattern.count_predicate = Some((op, count));
                            break;
                        }
                        _ => {
                            if let Some(dir) = super::helpers::parse_direction_ident(&id_low) {
                                self.advance();
                                let mut min_dist = 1;
                                let mut max_dist = None;
                                if let Some(Token::Number(n)) = self.peek() {
                                    min_dist = *n as usize;
                                    self.advance();
                                    if let Some(Token::Number(n2)) = self.peek() {
                                        max_dist = Some(*n2 as usize);
                                        self.advance();
                                    } else {
                                        max_dist = Some(min_dist);
                                    }
                                }
                                pattern.direction = Some((dir, min_dist, max_dist));
                            } else if let Some((color, role)) = parse_piece_specifier(id) {
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

        let cur_pos = self.current_pos();
        validate_move_pattern(&pattern, cur_pos)?;

        // Check if followed by `leads_to` / `leadsto` outcome block
        let is_leads_to = if let Some(Token::Ident(ref id)) = self.peek() {
            let id_low = id.to_lowercase();
            id_low == "leads_to" || id_low == "leadsto"
        } else {
            false
        };

        if is_leads_to {
            self.advance();
            let outcome_query = if let Some(Token::LBrace) = self.peek() {
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
            return Ok(SearchQuery::Play {
                move_pattern: pattern,
                outcome_query: Box::new(outcome_query),
            });
        }

        Ok(SearchQuery::Move(pattern))
    }
}
