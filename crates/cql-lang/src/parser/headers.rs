use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{HeaderPredicate, SearchQuery};

impl<'a> QueryParser<'a> {
    pub(crate) fn parse_header_keyword(
        &mut self,
        key: &str,
    ) -> Result<Option<SearchQuery>, ParseError> {
        let query = match key {
            "player" => {
                self.advance();
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::Player {
                    name: val,
                    op: final_op,
                    case_sensitive: false,
                }))
            }
            "white" => {
                self.advance();
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::White {
                    name: val,
                    op: final_op,
                    case_sensitive: false,
                }))
            }
            "black" => {
                self.advance();
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::Black {
                    name: val,
                    op: final_op,
                    case_sensitive: false,
                }))
            }
            "elo" | "anyelo" | "any_elo" => {
                self.advance();
                let op = self.parse_comparison_op();
                let val = self.expect_number()? as u16;
                Some(SearchQuery::Header(HeaderPredicate::AnyElo {
                    op,
                    value: val,
                }))
            }
            "whiteelo" | "white_elo" => {
                self.advance();
                let op = self.parse_comparison_op();
                let val = self.expect_number()? as u16;
                Some(SearchQuery::Header(HeaderPredicate::WhiteElo {
                    op,
                    value: val,
                }))
            }
            "blackelo" | "black_elo" => {
                self.advance();
                let op = self.parse_comparison_op();
                let val = self.expect_number()? as u16;
                Some(SearchQuery::Header(HeaderPredicate::BlackElo {
                    op,
                    value: val,
                }))
            }
            "avgelo" | "avg_elo" => {
                self.advance();
                let op = self.parse_comparison_op();
                let val = self.expect_number()? as u16;
                Some(SearchQuery::Header(HeaderPredicate::AvgElo {
                    op,
                    value: val,
                }))
            }
            "elodiff" | "elo_diff" => {
                self.advance();
                let op = self.parse_comparison_op();
                let val = self.expect_number()? as i32;
                Some(SearchQuery::Header(HeaderPredicate::EloDiff {
                    op,
                    value: val,
                    absolute: true,
                }))
            }
            "raw_elodiff" | "raw_elo_diff" | "rawelodiff" => {
                self.advance();
                let op = self.parse_comparison_op();
                let val = self.expect_number()? as i32;
                Some(SearchQuery::Header(HeaderPredicate::EloDiff {
                    op,
                    value: val,
                    absolute: false,
                }))
            }
            "result" => {
                self.advance();
                let _ = self.parse_comparison_op();
                let val = self.expect_string_or_ident()?;
                Some(SearchQuery::Header(HeaderPredicate::Result {
                    expected: val,
                }))
            }
            "eco" => {
                self.advance();
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::Eco {
                    code: val,
                    op: final_op,
                }))
            }
            "date" | "year" => {
                self.advance();
                let op = self.parse_comparison_op();
                let mut val = String::new();
                if let Some(Token::Number(n)) = self.peek() {
                    val.push_str(&n.to_string());
                    self.advance();
                    while let Some(Token::Ident(s)) = self.peek() {
                        if s.starts_with('.') || s.starts_with('-') || s.starts_with('/') {
                            val.push_str(s);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                } else {
                    val = self.expect_string_or_ident()?;
                }
                Some(SearchQuery::Header(HeaderPredicate::Date {
                    op,
                    value: val,
                }))
            }
            "event" => {
                self.advance();
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::Event {
                    name: val,
                    op: final_op,
                    case_sensitive: false,
                }))
            }
            "site" => {
                self.advance();
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::Site {
                    name: val,
                    op: final_op,
                    case_sensitive: false,
                }))
            }
            "tag" | "header" => {
                self.advance();
                let tag_name = self.expect_string_or_ident()?;
                let op = self.parse_string_comparison_op();
                let (val, final_op) = self.parse_string_or_regex_val(op)?;
                Some(SearchQuery::Header(HeaderPredicate::Tag {
                    name: tag_name,
                    op: final_op,
                    value: val,
                    case_sensitive: false,
                }))
            }
            _ => None,
        };

        Ok(query)
    }
}
