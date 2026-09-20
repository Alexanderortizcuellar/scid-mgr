/// Predicate for matching PGN and move annotations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationPredicate {
    /// Match text inside game/move comments
    Comment(CommentPredicate),
    /// Match Numeric Annotation Glyphs (NAGs) like !, ?, ??, !!, !?, ?!
    Nag(NagPredicate),
}

/// Predicate for matching comment contents
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentPredicate {
    /// Comment contains specified substring
    Contains { text: String, case_sensitive: bool },
    /// Comment starts with prefix
    StartsWith {
        prefix: String,
        case_sensitive: bool,
    },
    /// Comment matches a regular expression pattern
    Regex(String),
    /// Position or move has any non-empty comment
    HasComment,
    /// Position or move has no comment
    NoComment,
}

impl CommentPredicate {
    /// Evaluate the comment predicate against a comment string
    pub fn matches(&self, comment: &str) -> bool {
        match self {
            CommentPredicate::Contains {
                text,
                case_sensitive,
            } => {
                if *case_sensitive {
                    comment.contains(text)
                } else {
                    comment.to_lowercase().contains(&text.to_lowercase())
                }
            }
            CommentPredicate::StartsWith {
                prefix,
                case_sensitive,
            } => {
                if *case_sensitive {
                    comment.starts_with(prefix)
                } else {
                    comment.to_lowercase().starts_with(&prefix.to_lowercase())
                }
            }
            CommentPredicate::Regex(pattern) => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(comment)
                } else {
                    false
                }
            }
            CommentPredicate::HasComment => !comment.trim().is_empty(),
            CommentPredicate::NoComment => comment.trim().is_empty(),
        }
    }
}

/// Predicate for matching Numeric Annotation Glyphs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NagPredicate {
    /// Contains any of the specified NAG codes (e.g. 1 = !, 2 = ?, 3 = !!, 4 = ??)
    AnyOf(Vec<u8>),
    /// Move has any NAG annotation attached
    HasAnyNag,
    /// Move has no NAG annotation
    NoNag,
}

impl NagPredicate {
    pub fn matches(&self, nags: &[u8]) -> bool {
        match self {
            NagPredicate::AnyOf(target_nags) => nags.iter().any(|nag| target_nags.contains(nag)),
            NagPredicate::HasAnyNag => !nags.is_empty(),
            NagPredicate::NoNag => nags.is_empty(),
        }
    }

    /// Convert common symbol to standard NAG code
    pub fn symbol_to_nag(symbol: &str) -> Option<u8> {
        let trimmed = symbol.trim();
        if let Some(stripped) = trimmed.strip_prefix('$') {
            if let Ok(num) = stripped.parse::<u8>() {
                return Some(num);
            }
        }
        if let Ok(num) = trimmed.parse::<u8>() {
            return Some(num);
        }
        match trimmed {
            "!" => Some(1),
            "?" => Some(2),
            "!!" => Some(3),
            "??" => Some(4),
            "!?" => Some(5),
            "?!" => Some(6),
            "□" | "only move" => Some(7),
            "=" | "draw" => Some(10),
            "+=" | "⩲" => Some(14),
            "=+" | "⩱" => Some(15),
            "+/-" | "±" => Some(16),
            "-/+" | "∓" => Some(17),
            "+-" => Some(18),
            "-+" => Some(19),
            "N" | "novelty" => Some(146),
            _ => None,
        }
    }
}

/// Annotation and comment utilities for PGN/move streams
pub struct AnnotationManager;

impl AnnotationManager {
    /// Strip all comments in `{...}` and NAG annotations `$1..$255` from a PGN move text
    pub fn strip_comments(pgn_text: &str) -> String {
        let mut result = String::with_capacity(pgn_text.len());
        let mut in_comment = false;

        for ch in pgn_text.chars() {
            if ch == '{' {
                in_comment = true;
                continue;
            }
            if ch == '}' {
                in_comment = false;
                continue;
            }
            if !in_comment {
                result.push(ch);
            }
        }

        result
    }

    /// Extract all comments found in `{...}` blocks along with their approximate textual position
    pub fn extract_comments(pgn_text: &str) -> Vec<String> {
        let mut comments = Vec::new();
        let mut current_comment = String::new();
        let mut in_comment = false;

        for ch in pgn_text.chars() {
            if ch == '{' {
                in_comment = true;
                current_comment.clear();
                continue;
            }
            if ch == '}' {
                in_comment = false;
                comments.push(current_comment.trim().to_string());
                continue;
            }
            if in_comment {
                current_comment.push(ch);
            }
        }

        comments
    }
}
