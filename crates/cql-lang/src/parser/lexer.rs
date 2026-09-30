/// Error returned during CQL DSL query parsing with rich diagnostics for GUI and CLI debugging
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ParseError {
    /// Human-readable explanation of the error
    pub message: String,
    /// Absolute byte offset in the input string
    pub position: usize,
    /// 1-indexed line number where the error occurred
    pub line: usize,
    /// 1-indexed column number within the line
    pub column: usize,
    /// Visual source code snippet highlighting the error with carets (^)
    pub snippet: Option<String>,
    /// Actionable tip or suggestion on how to fix the error
    pub help: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
            line: 1,
            column: position + 1,
            snippet: None,
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Augment error with 1-indexed line, column, visual caret snippet, and smart hints
    pub fn with_source_context(mut self, input: &str) -> Self {
        let (line_num, col_num, line_str) = calculate_line_col_snippet(input, self.position);
        self.line = line_num;
        self.column = col_num;

        let line_num_str = format!("{}", line_num);
        let padding = " ".repeat(line_num_str.len());
        let caret_indent = " ".repeat(col_num.saturating_sub(1));

        let snippet = format!(
            " {} | {}\n {} | {}{}",
            line_num_str, line_str, padding, caret_indent, "^"
        );
        self.snippet = Some(snippet);

        if self.help.is_none() {
            self.help = generate_smart_help(&self.message, input, self.position);
        }

        self
    }
}

fn calculate_line_col_snippet(input: &str, byte_offset: usize) -> (usize, usize, String) {
    let mut line_num = 1;
    let mut col_num = 1;
    let mut line_start = 0;

    let clamped_offset = byte_offset.min(input.len());

    for (idx, ch) in input.char_indices() {
        if idx >= clamped_offset {
            break;
        }
        if ch == '\n' {
            line_num += 1;
            col_num = 1;
            line_start = idx + 1;
        } else {
            col_num += 1;
        }
    }

    let line_end = input[line_start..]
        .find('\n')
        .map(|idx| line_start + idx)
        .unwrap_or(input.len());
    let line_str = input[line_start..line_end]
        .trim_end_matches('\r')
        .to_string();

    (line_num, col_num, line_str)
}

fn generate_smart_help(msg: &str, _input: &str, _pos: usize) -> Option<String> {
    let msg_low = msg.to_lowercase();
    if msg_low.contains("square") {
        Some("Expected a valid square (e.g. 'e4', 'd8'), range (e.g. 'a1..h8', 'a1-h2'), or set ('light', 'dark').".to_string())
    } else if msg_low.contains("piece") {
        Some("Expected a piece symbol (e.g. 'B', 'N', 'R', 'Q', 'K', 'P', 'b', 'n', 'r', 'q', 'k', 'p', 'A', 'a', 'white_pieces', 'black_pieces').".to_string())
    } else if msg_low.contains("fen") {
        Some("FEN patterns can be full FENs or rank wildcards with '*', '?', 'A', 'a' (e.g. fen '*/*/*/*ppA*/*/*/*/*').".to_string())
    } else if msg_low.contains("')'") {
        Some("Check for missing closing parenthesis ')' matching an opening '('.".to_string())
    } else if msg_low.contains("']'") {
        Some("Check for missing closing bracket ']' matching an opening '['.".to_string())
    } else if msg_low.contains("comparison") || msg_low.contains("operator") {
        Some("Supported comparison operators: '==', '!=', '>=', '<=', '>', '<', ':'.".to_string())
    } else if msg_low.contains("trailing") {
        Some("Make sure statements are separated by newlines, 'and', or parentheses.".to_string())
    } else {
        None
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Syntax error at line {}, col {}:\n{}\nError: {}",
            self.line,
            self.column,
            self.snippet.as_deref().unwrap_or(""),
            self.message
        )?;
        if let Some(ref h) = self.help {
            write!(f, "\nHint: {}", h)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    StringLit(String),
    Number(i64),
    // Operators
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Colon,
    Comma,
    Tilde,      // "~"
    Pipe,       // "|"
    Ampersand,  // "&"
    Backslash,  // "\"
    Dots,       // "..."
    DotDot,     // ".."
    ArrowRight, // "-->" or "->"
    ArrowLeft,  // "<--" or "<-"
    // Brackets & Parentheses
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
}

/// Tokenizer for CQL-like text
pub struct Lexer<'a> {
    input: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().collect(),
            pos: 0,
        }
    }

    pub fn current_pos(&self) -> usize {
        if self.pos < self.chars.len() {
            self.chars[self.pos].0
        } else {
            self.input.len()
        }
    }

    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|&(_, c)| c)
    }

    pub fn advance(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos].1;
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<(usize, Token)>, ParseError> {
        let mut tokens = Vec::new();

        while let Some(c) = self.peek() {
            let start = self.current_pos();

            if c.is_whitespace() {
                self.advance();
                continue;
            }

            // Line comments (//)
            if c == '/' && self.pos + 1 < self.chars.len() && self.chars[self.pos + 1].1 == '/' {
                while let Some(ch) = self.peek() {
                    self.advance();
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }

            // Block comments (/* ... */)
            if c == '/' && self.pos + 1 < self.chars.len() && self.chars[self.pos + 1].1 == '*' {
                self.advance(); // consume '/'
                self.advance(); // consume '*'
                while let Some(ch) = self.peek() {
                    if ch == '*'
                        && self.pos + 1 < self.chars.len()
                        && self.chars[self.pos + 1].1 == '/'
                    {
                        self.advance(); // consume '*'
                        self.advance(); // consume '/'
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            match c {
                '(' => {
                    self.advance();
                    tokens.push((start, Token::LParen));
                }
                ')' => {
                    self.advance();
                    tokens.push((start, Token::RParen));
                }
                '[' => {
                    self.advance();
                    tokens.push((start, Token::LBracket));
                }
                ']' => {
                    self.advance();
                    tokens.push((start, Token::RBracket));
                }
                '{' => {
                    self.advance();
                    tokens.push((start, Token::LBrace));
                }
                '}' => {
                    self.advance();
                    tokens.push((start, Token::RBrace));
                }
                ':' => {
                    self.advance();
                    tokens.push((start, Token::Colon));
                }
                ',' => {
                    self.advance();
                    tokens.push((start, Token::Comma));
                }
                '~' => {
                    self.advance();
                    tokens.push((start, Token::Tilde));
                }
                '|' => {
                    self.advance();
                    tokens.push((start, Token::Pipe));
                }
                '&' => {
                    self.advance();
                    tokens.push((start, Token::Ampersand));
                }
                '\\' => {
                    self.advance();
                    tokens.push((start, Token::Backslash));
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                    }
                    tokens.push((start, Token::Eq));
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push((start, Token::Neq));
                    } else if self.peek() == Some('!') {
                        self.advance();
                        tokens.push((start, Token::Ident("!!".to_string())));
                    } else if self.peek() == Some('?') {
                        self.advance();
                        tokens.push((start, Token::Ident("!?".to_string())));
                    } else {
                        tokens.push((start, Token::Ident("!".to_string())));
                    }
                }
                '?' => {
                    self.advance();
                    if self.peek() == Some('?') {
                        self.advance();
                        tokens.push((start, Token::Ident("??".to_string())));
                    } else if self.peek() == Some('!') {
                        self.advance();
                        tokens.push((start, Token::Ident("?!".to_string())));
                    } else {
                        tokens.push((start, Token::Ident("?".to_string())));
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        tokens.push((start, Token::Gte));
                    } else {
                        tokens.push((start, Token::Gt));
                    }
                }
                '<' => {
                    if self.pos + 2 < self.chars.len()
                        && self.chars[self.pos + 1].1 == '-'
                        && self.chars[self.pos + 2].1 == '-'
                    {
                        self.advance();
                        self.advance();
                        self.advance();
                        tokens.push((start, Token::ArrowLeft));
                    } else if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1].1 == '-' {
                        self.advance();
                        self.advance();
                        tokens.push((start, Token::ArrowLeft));
                    } else {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push((start, Token::Lte));
                        } else if self.peek() == Some('>') {
                            self.advance();
                            tokens.push((start, Token::Neq));
                        } else {
                            tokens.push((start, Token::Lt));
                        }
                    }
                }
                '.' => {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            tokens.push((start, Token::Dots));
                        } else {
                            tokens.push((start, Token::DotDot));
                        }
                    } else {
                        // Single dot - treat as ident token or part of move number
                        tokens.push((start, Token::Ident(".".to_string())));
                    }
                }
                '"' | '\'' => {
                    let quote = c;
                    self.advance();
                    let mut s = String::new();
                    while let Some(ch) = self.peek() {
                        self.advance();
                        if ch == quote {
                            break;
                        }
                        if ch == '\\' {
                            if let Some(escaped) = self.advance() {
                                s.push(escaped);
                                continue;
                            }
                        }
                        s.push(ch);
                    }
                    tokens.push((start, Token::StringLit(s)));
                }
                _ if c == '-'
                    && self.pos + 2 < self.chars.len()
                    && self.chars[self.pos + 1].1 == '-'
                    && self.chars[self.pos + 2].1 == '>' =>
                {
                    self.advance();
                    self.advance();
                    self.advance();
                    tokens.push((start, Token::ArrowRight));
                }
                _ if c == '-'
                    && self.pos + 1 < self.chars.len()
                    && self.chars[self.pos + 1].1 == '>' =>
                {
                    self.advance();
                    self.advance();
                    tokens.push((start, Token::ArrowRight));
                }
                _ if c.is_ascii_digit() || (c == '-' && self.is_next_digit()) => {
                    let mut num_str = String::new();
                    if c == '-' {
                        num_str.push('-');
                        self.advance();
                    }
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_digit() {
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let num = num_str
                        .parse::<i64>()
                        .map_err(|e| ParseError::new(format!("Invalid number: {}", e), start))?;
                    tokens.push((start, Token::Number(num)));
                }
                _ if is_ident_start(c) => {
                    let mut ident = String::new();
                    while let Some(ch) = self.peek() {
                        if is_ident_char(ch) {
                            ident.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    tokens.push((start, Token::Ident(ident)));
                }
                _ => {
                    self.advance();
                    return Err(ParseError::new(
                        format!("Unexpected character: {:?}", c),
                        start,
                    ));
                }
            }
        }

        Ok(tokens)
    }

    fn is_next_digit(&self) -> bool {
        if self.pos + 1 < self.chars.len() {
            self.chars[self.pos + 1].1.is_ascii_digit()
        } else {
            false
        }
    }
}

pub fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$' || c == '+' || c == '#' || c == '-' || c == '*'
}

pub fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || c == '$'
        || c == '-'
        || c == '+'
        || c == '#'
        || c == '?'
        || c == '!'
        || c == '/'
        || c == '*'
}
