use std::ops::Range;

use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{ComparisonOp, PositionPattern, SearchQuery};

impl<'a> QueryParser<'a> {
    pub(crate) fn parse_ply_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();

        // Check if comparison op follows: ply == 10, ply <= 20, ply > 5
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

        if is_cmp {
            let op = self.parse_comparison_op();
            let val = self.expect_number()? as usize;
            return Ok(SearchQuery::Position(PositionPattern::Ply {
                op,
                value: val,
            }));
        }

        self.match_ident("in");

        if let Some(Token::Number(n)) = self.peek() {
            let start = *n as usize;
            self.advance();
            let mut end = usize::MAX;

            if let Some(Token::DotDot) = self.peek() {
                self.advance();
                if let Some(Token::Number(n2)) = self.peek() {
                    end = *n2 as usize;
                    self.advance();
                }
            } else if let Some(Token::Number(n2)) = self.peek() {
                end = *n2 as usize;
                self.advance();
            }

            if let Some(Token::LParen) | Some(Token::LBrace) = self.peek() {
                let is_brace = matches!(self.peek(), Some(Token::LBrace));
                self.advance();
                let sub_query = self.parse_or_expr()?;
                if is_brace {
                    if let Some(Token::RBrace) = self.peek() {
                        self.advance();
                    }
                } else if let Some(Token::RParen) = self.peek() {
                    self.advance();
                }
                return Ok(SearchQuery::PlyRange {
                    range: Range { start, end },
                    query: Box::new(sub_query),
                });
            }

            // Standalone ply range: ply 1..20 -> ply >= 1 and ply <= 20
            if end == usize::MAX {
                return Ok(SearchQuery::Position(PositionPattern::Ply {
                    op: ComparisonOp::Equal,
                    value: start,
                }));
            } else {
                return Ok(SearchQuery::And(vec![
                    SearchQuery::Position(PositionPattern::Ply {
                        op: ComparisonOp::GreaterThanOrEqual,
                        value: start,
                    }),
                    SearchQuery::Position(PositionPattern::Ply {
                        op: ComparisonOp::LessThanOrEqual,
                        value: end,
                    }),
                ]));
            }
        }

        Err(ParseError::new(
            "Expected comparison operator or range after ply".to_string(),
            pos,
        ))
    }

    pub(crate) fn parse_move_number_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let op = self.parse_comparison_op();
        let val = self.expect_number()? as usize;
        Ok(SearchQuery::Position(PositionPattern::MoveNumber {
            op,
            value: val,
        }))
    }

    pub(crate) fn parse_occurrences_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        let (min, max) = if matches!(
            self.peek(),
            Some(Token::Eq | Token::Gt | Token::Gte | Token::Lt | Token::Lte | Token::Colon)
        ) {
            let op = self.parse_comparison_op();
            let count = self.expect_number()? as usize;
            match op {
                ComparisonOp::Equal => (count, count),
                ComparisonOp::GreaterThan => (count + 1, usize::MAX),
                ComparisonOp::GreaterThanOrEqual => (count, usize::MAX),
                ComparisonOp::LessThan => (0, count.saturating_sub(1)),
                ComparisonOp::LessThanOrEqual => (0, count),
                ComparisonOp::NotEqual => (count, count),
                _ => (count, count),
            }
        } else if let Some(Token::Number(n)) = self.peek() {
            let start = *n as usize;
            self.advance();
            let mut end = start;
            if let Some(Token::DotDot) = self.peek() {
                self.advance();
                if let Some(Token::Number(n2)) = self.peek() {
                    end = *n2 as usize;
                    self.advance();
                } else {
                    end = usize::MAX;
                }
            }
            (start, end)
        } else {
            return Err(ParseError::new(
                "Expected count or range after occurrences / count".to_string(),
                pos,
            ));
        };

        if let Some(Token::LParen) | Some(Token::LBrace) = self.peek() {
            let is_brace = matches!(self.peek(), Some(Token::LBrace));
            self.advance();
            let sub_query = self.parse_or_expr()?;
            if is_brace {
                if let Some(Token::RBrace) = self.peek() {
                    self.advance();
                }
            } else if let Some(Token::RParen) = self.peek() {
                self.advance();
            }
            return Ok(SearchQuery::Occurrences {
                min,
                max: if max == usize::MAX { None } else { Some(max) },
                query: Box::new(sub_query),
            });
        }

        Err(ParseError::new(
            "Expected '(' or '{' after occurrences expression".to_string(),
            pos,
        ))
    }
}
