use shakmaty::{Color, File, Role, Square};
use std::str::FromStr;

use super::helpers::{parse_piece_specifier, parse_square_or_piece};
use super::lexer::{ParseError, Token};
use super::QueryParser;
use crate::search::query::{PieceMatcher, SearchQuery, TacticalPredicate};

impl<'a> QueryParser<'a> {
    pub(crate) fn parse_piece_matcher_arg(&mut self) -> Result<Vec<PieceMatcher>, ParseError> {
        let pos = self.current_pos();
        if let Some(Token::LBracket) | Some(Token::LParen) = self.peek() {
            let matchers = self.parse_piece_matcher_list();
            return Ok(matchers);
        }
        if let Some(Token::Ident(ref id)) = self.peek() {
            let id_clone = id.clone();
            self.advance();
            if let Some((c, r)) = parse_piece_specifier(&id_clone) {
                let color = if id_clone.len() == 1
                    && id_clone.chars().next().unwrap().is_ascii_uppercase()
                {
                    None
                } else {
                    c
                };
                return Ok(vec![PieceMatcher::new(color, r)]);
            } else {
                return Err(ParseError::new(
                    format!("Invalid piece specifier: {}", id_clone),
                    pos,
                ));
            }
        }
        Ok(Vec::new())
    }

    pub(crate) fn parse_pin_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let mut pinners = Vec::new();
        let mut pinneds = Vec::new();
        let mut targets = Vec::new();

        if let Some(Token::LParen) | Some(Token::LBracket) = self.peek() {
            let is_bracket = matches!(self.peek(), Some(Token::LBracket));
            self.advance();

            let mut positional_idx = 0;
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

                if let Token::Ident(ref id) = tok {
                    let id_low = id.to_lowercase();
                    match id_low.as_str() {
                        "from" | "pinner" | "attacker" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            pinners.extend(m);
                        }
                        "through" | "pinned" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            pinneds.extend(m);
                        }
                        "to" | "target" | "targets" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            targets.extend(m);
                        }
                        _ => {
                            if let Ok(m) = self.parse_piece_matcher_arg() {
                                if !m.is_empty() {
                                    match positional_idx {
                                        0 => pinners.extend(m),
                                        1 => pinneds.extend(m),
                                        2 => targets.extend(m),
                                        _ => {}
                                    }
                                    positional_idx += 1;
                                }
                            } else {
                                self.advance();
                            }
                        }
                    }
                } else if let Some(Token::LBracket) = self.peek() {
                    let m = self.parse_piece_matcher_list();
                    match positional_idx {
                        0 => pinners.extend(m),
                        1 => pinneds.extend(m),
                        2 => targets.extend(m),
                        _ => {}
                    }
                    positional_idx += 1;
                } else {
                    self.advance();
                }
            }
        } else {
            while let Some(Token::Ident(ref id)) = self.peek() {
                let id_low = id.to_lowercase();
                match id_low.as_str() {
                    "from" | "pinner" | "attacker" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        pinners.extend(m);
                    }
                    "to" | "target" | "targets" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        targets.extend(m);
                    }
                    "through" | "pinned" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        pinneds.extend(m);
                    }
                    _ => {
                        if let Ok(m) = self.parse_piece_matcher_arg() {
                            if !m.is_empty() {
                                pinners.extend(m);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(SearchQuery::Tactical(TacticalPredicate::Pin {
            pinners,
            pinneds,
            targets,
        }))
    }

    pub(crate) fn parse_fork_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let mut attackers = Vec::new();
        let mut target_slots: Vec<Vec<PieceMatcher>> = Vec::new();
        let mut targets_pool = Vec::new();
        let mut positional_args: Vec<Vec<PieceMatcher>> = Vec::new();
        let mut min_targets = 2;

        if let Some(Token::LParen) | Some(Token::LBracket) = self.peek() {
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

                if let Token::Ident(ref id) = tok {
                    let id_low = id.to_lowercase();
                    match id_low.as_str() {
                        "from" | "attacker" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            attackers.extend(m);
                        }
                        "to" | "target" | "target1" | "target2" | "target3" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            if !m.is_empty() {
                                target_slots.push(m);
                            }
                        }
                        "targets" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            if matches!(self.peek(), Some(Token::Ident(ref c_tok)) if c_tok.to_lowercase() == "count")
                            {
                                self.advance();
                                if matches!(
                                    self.peek(),
                                    Some(Token::Gte) | Some(Token::Gt) | Some(Token::Eq)
                                ) {
                                    self.advance();
                                    if let Some(Token::Number(n)) = self.peek() {
                                        min_targets = *n as usize;
                                        self.advance();
                                    }
                                }
                            } else {
                                let m = self.parse_piece_matcher_arg()?;
                                targets_pool.extend(m);
                            }
                        }
                        "count" => {
                            self.advance();
                            if matches!(
                                self.peek(),
                                Some(Token::Gte) | Some(Token::Gt) | Some(Token::Eq)
                            ) {
                                self.advance();
                                if let Some(Token::Number(n)) = self.peek() {
                                    min_targets = *n as usize;
                                    self.advance();
                                }
                            }
                        }
                        _ => {
                            if let Ok(m) = self.parse_piece_matcher_arg() {
                                if !m.is_empty() {
                                    positional_args.push(m);
                                }
                            } else {
                                self.advance();
                            }
                        }
                    }
                } else if let Some(Token::LBracket) = self.peek() {
                    let m = self.parse_piece_matcher_list();
                    if !m.is_empty() {
                        positional_args.push(m);
                    }
                } else {
                    self.advance();
                }
            }
        } else {
            while let Some(Token::Ident(ref id)) = self.peek() {
                let id_low = id.to_lowercase();
                match id_low.as_str() {
                    "from" | "attacker" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        attackers.extend(m);
                    }
                    "to" | "target" | "target1" | "target2" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        if !m.is_empty() {
                            target_slots.push(m);
                        }
                    }
                    "targets" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        targets_pool.extend(m);
                    }
                    _ => {
                        if let Ok(m) = self.parse_piece_matcher_arg() {
                            if !m.is_empty() {
                                positional_args.push(m);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Process positional arguments if present
        if !positional_args.is_empty() {
            if attackers.is_empty() {
                attackers.extend(positional_args[0].clone());
            }
            if positional_args.len() == 2 {
                let second = &positional_args[1];
                if second.len() == 1 {
                    target_slots.push(second.clone());
                } else {
                    targets_pool.extend(second.clone());
                }
            } else if positional_args.len() >= 3 {
                for slot in &positional_args[1..] {
                    target_slots.push(slot.clone());
                }
            }
        }

        Ok(SearchQuery::Tactical(TacticalPredicate::Fork {
            attackers,
            target_slots,
            targets_pool,
            min_targets,
        }))
    }

    pub(crate) fn parse_skewer_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let mut attackers = Vec::new();
        let mut fronts = Vec::new();
        let mut rears = Vec::new();

        if let Some(Token::LParen) | Some(Token::LBracket) = self.peek() {
            let is_bracket = matches!(self.peek(), Some(Token::LBracket));
            self.advance();

            let mut positional_idx = 0;
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

                if let Token::Ident(ref id) = tok {
                    let id_low = id.to_lowercase();
                    match id_low.as_str() {
                        "from" | "attacker" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            attackers.extend(m);
                        }
                        "through" | "front" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            fronts.extend(m);
                        }
                        "to" | "rear" => {
                            self.advance();
                            if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                                || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                            {
                                self.advance();
                            }
                            let m = self.parse_piece_matcher_arg()?;
                            rears.extend(m);
                        }
                        _ => {
                            if let Ok(m) = self.parse_piece_matcher_arg() {
                                if !m.is_empty() {
                                    match positional_idx {
                                        0 => attackers.extend(m),
                                        1 => fronts.extend(m),
                                        2 => rears.extend(m),
                                        _ => {}
                                    }
                                    positional_idx += 1;
                                }
                            } else {
                                self.advance();
                            }
                        }
                    }
                } else if let Some(Token::LBracket) = self.peek() {
                    let m = self.parse_piece_matcher_list();
                    match positional_idx {
                        0 => attackers.extend(m),
                        1 => fronts.extend(m),
                        2 => rears.extend(m),
                        _ => {}
                    }
                    positional_idx += 1;
                } else {
                    self.advance();
                }
            }
        } else {
            while let Some(Token::Ident(ref id)) = self.peek() {
                let id_low = id.to_lowercase();
                match id_low.as_str() {
                    "from" | "attacker" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        attackers.extend(m);
                    }
                    "through" | "front" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        fronts.extend(m);
                    }
                    "to" | "rear" => {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Ident(ref in_tok)) if in_tok.to_lowercase() == "in")
                            || matches!(self.peek(), Some(Token::Colon) | Some(Token::Eq))
                        {
                            self.advance();
                        }
                        let m = self.parse_piece_matcher_arg()?;
                        rears.extend(m);
                    }
                    _ => {
                        if let Ok(m) = self.parse_piece_matcher_arg() {
                            if !m.is_empty() {
                                attackers.extend(m);
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(SearchQuery::Tactical(TacticalPredicate::Skewer {
            attackers,
            fronts,
            rears,
        }))
    }

    pub(crate) fn parse_trapped_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let piece = if matches!(self.peek(), Some(Token::LParen) | Some(Token::LBracket)) {
            let matchers = self.parse_piece_matcher_list();
            matchers.first().copied().unwrap_or(PieceMatcher::ANY)
        } else if let Some(Token::Ident(ref s)) = self.peek() {
            let s_str = s.clone();
            if let Some((c, r)) = parse_piece_specifier(&s_str) {
                self.advance();
                PieceMatcher::new(c, r)
            } else {
                PieceMatcher::ANY
            }
        } else {
            PieceMatcher::ANY
        };

        Ok(SearchQuery::Tactical(TacticalPredicate::TrappedPiece {
            piece,
        }))
    }

    pub(crate) fn parse_outpost_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let mut piece = PieceMatcher::new(None, Some(Role::Knight));
        let mut square = None;

        if matches!(self.peek(), Some(Token::LBracket) | Some(Token::LParen)) {
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
                    let s_str = s.clone();
                    self.advance();
                    if let Ok(sq) = Square::from_str(&s_str.to_lowercase()) {
                        square = Some(sq);
                    } else if let Some((c, r)) = parse_piece_specifier(&s_str) {
                        piece = PieceMatcher::new(c, r);
                    }
                } else {
                    self.advance();
                }
            }

            return Ok(SearchQuery::Tactical(TacticalPredicate::Outpost {
                piece,
                square,
            }));
        }

        // Unbracketed: outpost knight on d5 / outpost white_knight on e5 / outpost d5 / outpost knight
        while let Some(Token::Ident(ref s)) = self.peek() {
            let lower = s.to_lowercase();
            if lower == "on" || lower == "at" || lower == "in" {
                self.advance();
                continue;
            }
            if let Ok(sq) = Square::from_str(&lower) {
                self.advance();
                square = Some(sq);
            } else if let Some((c, r)) = parse_piece_specifier(&lower) {
                self.advance();
                piece = PieceMatcher::new(c, r);
            } else {
                break;
            }
        }

        Ok(SearchQuery::Tactical(TacticalPredicate::Outpost {
            piece,
            square,
        }))
    }

    pub(crate) fn parse_open_file_expr(&mut self, semi: bool) -> Result<SearchQuery, ParseError> {
        let mut file = None;
        let mut semi_open_for = None;

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
                if let Token::Ident(s) = tok {
                    let lower = s.to_lowercase();
                    self.advance();
                    match lower.as_str() {
                        "a" => file = Some(File::A),
                        "c" => file = Some(File::C),
                        "d" => file = Some(File::D),
                        "e" => file = Some(File::E),
                        "f" => file = Some(File::F),
                        "g" => file = Some(File::G),
                        "h" => file = Some(File::H),
                        "white" | "w" => semi_open_for = Some(Color::White),
                        "black" => semi_open_for = Some(Color::Black),
                        "b" => {
                            if file.is_none() {
                                file = Some(File::B);
                            } else {
                                semi_open_for = Some(Color::Black);
                            }
                        }
                        _ => {}
                    }
                } else {
                    self.advance();
                }
            }
        }

        if !semi {
            Ok(SearchQuery::Tactical(TacticalPredicate::OpenFile {
                file,
                semi_open_for: None,
            }))
        } else {
            Ok(SearchQuery::Tactical(TacticalPredicate::OpenFile {
                file,
                semi_open_for,
            }))
        }
    }

    pub(crate) fn parse_distance_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        if let Some(Token::LParen) = self.peek() {
            self.advance();
            let first_token = self.expect_ident()?;
            if let Some(Token::Comma) = self.peek() {
                self.advance();
            }
            let second_token = self.expect_ident()?;
            self.expect_token(Token::RParen)?;

            let sq1 = parse_square_or_piece(&first_token).ok_or_else(|| {
                ParseError::new(
                    format!("Invalid square or piece in distance: {}", first_token),
                    pos,
                )
            })?;
            let sq2 = parse_square_or_piece(&second_token).ok_or_else(|| {
                ParseError::new(
                    format!("Invalid square or piece in distance: {}", second_token),
                    pos,
                )
            })?;

            let op = self.parse_comparison_op();
            let distance = self.expect_number()? as usize;

            return Ok(SearchQuery::Tactical(TacticalPredicate::Distance {
                sq1,
                sq2,
                op,
                distance,
            }));
        }

        Err(ParseError::new(
            "Expected '(' after distance".to_string(),
            pos,
        ))
    }

    pub(crate) fn parse_attacks_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        if let Some(Token::LParen) | Some(Token::LBracket) = self.peek() {
            let is_bracket = matches!(self.peek(), Some(Token::LBracket));
            self.advance();

            let first_token = self.expect_ident()?;
            if let Some(Token::Comma) = self.peek() {
                self.advance();
            }
            let second_token = self.expect_ident()?;

            if is_bracket {
                self.expect_token(Token::RBracket)?;
            } else {
                self.expect_token(Token::RParen)?;
            }

            let attacker = parse_square_or_piece(&first_token).ok_or_else(|| {
                ParseError::new(format!("Invalid attacker in attacks: {}", first_token), pos)
            })?;
            let target = parse_square_or_piece(&second_token).ok_or_else(|| {
                ParseError::new(format!("Invalid target in attacks: {}", second_token), pos)
            })?;

            return Ok(SearchQuery::Tactical(TacticalPredicate::Attacks {
                attacker,
                target,
            }));
        }

        // Direct syntax: attacks $n k, attacks N to q, attacks N against k (or attacks N k)
        let first_token = self.expect_ident()?;
        if let Some(Token::Ident(ref s)) = self.peek() {
            let s_low = s.to_lowercase();
            if s_low == "to" || s_low == "against" || s_low == "on" {
                self.advance();
            }
        }
        let second_token = self.expect_ident()?;

        let attacker = parse_square_or_piece(&first_token).ok_or_else(|| {
            ParseError::new(format!("Invalid attacker in attacks: {}", first_token), pos)
        })?;
        let target = parse_square_or_piece(&second_token).ok_or_else(|| {
            ParseError::new(format!("Invalid target in attacks: {}", second_token), pos)
        })?;

        Ok(SearchQuery::Tactical(TacticalPredicate::Attacks {
            attacker,
            target,
        }))
    }

    pub(crate) fn parse_attacked_expr(&mut self) -> Result<SearchQuery, ParseError> {
        let pos = self.current_pos();
        if let Some(Token::LParen) | Some(Token::LBracket) = self.peek() {
            let is_bracket = matches!(self.peek(), Some(Token::LBracket));
            self.advance();

            let target_token = self.expect_ident()?;
            if let Some(Token::Comma) = self.peek() {
                self.advance();
            }
            let attacker_token = self.expect_ident()?;

            if is_bracket {
                self.expect_token(Token::RBracket)?;
            } else {
                self.expect_token(Token::RParen)?;
            }

            let target = parse_square_or_piece(&target_token).ok_or_else(|| {
                ParseError::new(format!("Invalid target in attacked: {}", target_token), pos)
            })?;
            let attacker = parse_square_or_piece(&attacker_token).ok_or_else(|| {
                ParseError::new(
                    format!("Invalid attacker in attacked: {}", attacker_token),
                    pos,
                )
            })?;

            return Ok(SearchQuery::Tactical(TacticalPredicate::Attacks {
                attacker,
                target,
            }));
        }

        // Direct syntax: is_attacked e4 by black, attacked e4 from B, attacked e4 by white
        let target_token = self.expect_ident()?;
        if let Some(Token::Ident(ref s)) = self.peek() {
            let s_low = s.to_lowercase();
            if s_low == "by" || s_low == "from" || s_low == "with" {
                self.advance();
            }
        }
        let attacker_token = self.expect_ident()?;

        let target = parse_square_or_piece(&target_token).ok_or_else(|| {
            ParseError::new(format!("Invalid target in attacked: {}", target_token), pos)
        })?;
        let attacker = parse_square_or_piece(&attacker_token).ok_or_else(|| {
            ParseError::new(
                format!("Invalid attacker in attacked: {}", attacker_token),
                pos,
            )
        })?;

        Ok(SearchQuery::Tactical(TacticalPredicate::Attacks {
            attacker,
            target,
        }))
    }

    pub(crate) fn parse_piece_matcher_list(&mut self) -> Vec<PieceMatcher> {
        let mut list = Vec::new();

        if let Some(Token::LBracket) | Some(Token::LParen) = self.peek() {
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
                    let s_clone = s.clone();
                    self.advance();
                    if let Some((c, r)) = parse_piece_specifier(&s_clone) {
                        list.push(PieceMatcher::new(c, r));
                    }
                } else {
                    self.advance();
                }
            }
        }

        list
    }
}
