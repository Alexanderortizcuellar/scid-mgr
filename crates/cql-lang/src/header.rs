use super::query::{ComparisonOp, HeaderPredicate};
use std::collections::HashMap;

/// Header metadata matching engine
pub struct HeaderMatcher;

impl HeaderMatcher {
    /// Evaluate a single HeaderPredicate against a map of PGN headers (case-insensitive keys)
    pub fn matches(predicate: &HeaderPredicate, headers: &HashMap<String, String>) -> bool {
        match predicate {
            HeaderPredicate::Tag {
                name,
                op,
                value,
                case_sensitive,
            } => {
                let actual = get_header_value(headers, name);
                compare_string(actual, value, *op, *case_sensitive)
            }
            HeaderPredicate::Player {
                name,
                op,
                case_sensitive,
            } => {
                let white = get_header_value(headers, "White");
                let black = get_header_value(headers, "Black");
                compare_string(white, name, *op, *case_sensitive)
                    || compare_string(black, name, *op, *case_sensitive)
            }
            HeaderPredicate::White {
                name,
                op,
                case_sensitive,
            } => {
                let white = get_header_value(headers, "White");
                compare_string(white, name, *op, *case_sensitive)
            }
            HeaderPredicate::Black {
                name,
                op,
                case_sensitive,
            } => {
                let black = get_header_value(headers, "Black");
                compare_string(black, name, *op, *case_sensitive)
            }
            HeaderPredicate::WhiteElo { op, value } => {
                let elo = parse_u16(get_header_value(headers, "WhiteElo"));
                compare_numeric(elo, Some(*value), *op)
            }
            HeaderPredicate::BlackElo { op, value } => {
                let elo = parse_u16(get_header_value(headers, "BlackElo"));
                compare_numeric(elo, Some(*value), *op)
            }
            HeaderPredicate::AnyElo { op, value } => {
                let w_elo = parse_u16(get_header_value(headers, "WhiteElo"));
                let b_elo = parse_u16(get_header_value(headers, "BlackElo"));
                compare_numeric(w_elo, Some(*value), *op)
                    || compare_numeric(b_elo, Some(*value), *op)
            }
            HeaderPredicate::AvgElo { op, value } => {
                let w_elo = parse_u16(get_header_value(headers, "WhiteElo"));
                let b_elo = parse_u16(get_header_value(headers, "BlackElo"));
                match (w_elo, b_elo) {
                    (Some(w), Some(b)) => {
                        let avg = (w + b) / 2;
                        compare_numeric(Some(avg), Some(*value), *op)
                    }
                    _ => false,
                }
            }
            HeaderPredicate::EloDiff {
                op,
                value,
                absolute,
            } => {
                let w_elo = parse_u16(get_header_value(headers, "WhiteElo"));
                let b_elo = parse_u16(get_header_value(headers, "BlackElo"));
                match (w_elo, b_elo) {
                    (Some(w), Some(b)) => {
                        let diff = if *absolute {
                            (w as i32 - b as i32).abs()
                        } else {
                            w as i32 - b as i32
                        };
                        compare_numeric(Some(diff), Some(*value), *op)
                    }
                    _ => false,
                }
            }
            HeaderPredicate::Result { expected } => {
                let actual = get_header_value(headers, "Result");
                if expected == "All" || expected == "*" && actual.is_empty() {
                    return true;
                }
                actual == expected
            }
            HeaderPredicate::Eco { code, op } => {
                let actual = get_header_value(headers, "ECO");
                compare_string(actual, code, *op, false)
            }
            HeaderPredicate::Date { op, value } => {
                let actual = get_header_value(headers, "Date");
                compare_date(actual, value, *op)
            }
            HeaderPredicate::Event {
                name,
                op,
                case_sensitive,
            } => {
                let event = get_header_value(headers, "Event");
                compare_string(event, name, *op, *case_sensitive)
            }
            HeaderPredicate::Site {
                name,
                op,
                case_sensitive,
            } => {
                let site = get_header_value(headers, "Site");
                compare_string(site, name, *op, *case_sensitive)
            }
            HeaderPredicate::Round { value } => {
                let actual = get_header_value(headers, "Round");
                actual.trim() == value.trim()
            }
            HeaderPredicate::Custom { key, op, value } => {
                let actual = get_header_value(headers, key);
                compare_string(actual, value, *op, false)
            }
        }
    }
}

pub fn get_header_value<'a>(headers: &'a HashMap<String, String>, key: &str) -> &'a str {
    if let Some(val) = headers.get(key) {
        return val.as_str();
    }
    // Case-insensitive fallback
    let key_lower = key.to_lowercase();
    for (k, v) in headers {
        if k.to_lowercase() == key_lower {
            return v.as_str();
        }
    }
    ""
}

pub fn parse_u16(s: &str) -> Option<u16> {
    s.trim().parse::<u16>().ok()
}

pub fn compare_numeric<T: Ord>(actual: Option<T>, expected: Option<T>, op: ComparisonOp) -> bool {
    let (a, e) = match (actual, expected) {
        (Some(a), Some(e)) => (a, e),
        _ => return false,
    };

    match op {
        ComparisonOp::Equal => a == e,
        ComparisonOp::NotEqual => a != e,
        ComparisonOp::GreaterThan => a > e,
        ComparisonOp::GreaterThanOrEqual => a >= e,
        ComparisonOp::LessThan => a < e,
        ComparisonOp::LessThanOrEqual => a <= e,
        _ => false,
    }
}

pub fn compare_string(
    actual: &str,
    expected: &str,
    op: ComparisonOp,
    case_sensitive: bool,
) -> bool {
    let (a, e) = if case_sensitive {
        (actual.to_string(), expected.to_string())
    } else {
        (actual.to_lowercase(), expected.to_lowercase())
    };

    match op {
        ComparisonOp::Equal => a == e,
        ComparisonOp::NotEqual => a != e,
        ComparisonOp::Contains => a.contains(&e),
        ComparisonOp::StartsWith => a.starts_with(&e),
        ComparisonOp::EndsWith => a.ends_with(&e),
        ComparisonOp::GreaterThan => a > e,
        ComparisonOp::GreaterThanOrEqual => a >= e,
        ComparisonOp::LessThan => a < e,
        ComparisonOp::LessThanOrEqual => a <= e,
        ComparisonOp::Regex => {
            if let Ok(re) = regex::RegexBuilder::new(expected)
                .case_insensitive(!case_sensitive)
                .build()
            {
                re.is_match(actual)
            } else {
                wildcard_match(&a, &e)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedDate {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
}

impl ParsedDate {
    pub fn parse(s: &str) -> Self {
        let trimmed = s.trim().trim_matches('"').trim_matches('\'');
        // Split by '.', '-', or '/'
        let parts: Vec<&str> = trimmed
            .split(['.', '-', '/'])
            .filter(|p| !p.is_empty())
            .collect();

        let mut year = None;
        let mut month = None;
        let mut day = None;

        if let Some(y_str) = parts.first() {
            let y_cleaned: String = y_str.chars().filter(|c| c.is_ascii_digit()).collect();
            if !y_cleaned.is_empty() {
                year = y_cleaned.parse::<i32>().ok();
            }
        }

        if let Some(m_str) = parts.get(1) {
            let m_cleaned: String = m_str.chars().filter(|c| c.is_ascii_digit()).collect();
            if !m_cleaned.is_empty() {
                month = m_cleaned.parse::<u32>().ok();
            }
        }

        if let Some(d_str) = parts.get(2) {
            let d_cleaned: String = d_str.chars().filter(|c| c.is_ascii_digit()).collect();
            if !d_cleaned.is_empty() {
                day = d_cleaned.parse::<u32>().ok();
            }
        }

        Self { year, month, day }
    }

    /// Convert to a comparable tuple with wildcards/unknowns resolved
    pub fn to_bound(self, is_upper_bound: bool) -> Option<(i32, u32, u32)> {
        let y = self.year?;
        let m = self.month.unwrap_or(if is_upper_bound { 12 } else { 1 });
        let d = self.day.unwrap_or(if is_upper_bound { 31 } else { 1 });
        Some((y, m, d))
    }
}

pub fn compare_date(actual: &str, expected: &str, op: ComparisonOp) -> bool {
    let act = ParsedDate::parse(actual);
    let exp = ParsedDate::parse(expected);

    // If both could not parse a valid year, fallback to standard string comparison
    if act.year.is_none() || exp.year.is_none() {
        return compare_string(actual, expected, op, false);
    }

    match op {
        ComparisonOp::Equal => {
            // For Equal: match specified components (e.g. if expected is Year only, match Year)
            if let Some(exp_y) = exp.year {
                if act.year != Some(exp_y) {
                    return false;
                }
            }
            if let Some(exp_m) = exp.month {
                if act.month != Some(exp_m) {
                    return false;
                }
            }
            if let Some(exp_d) = exp.day {
                if act.day != Some(exp_d) {
                    return false;
                }
            }
            true
        }
        ComparisonOp::NotEqual => !compare_date(actual, expected, ComparisonOp::Equal),
        ComparisonOp::GreaterThan => match (act.to_bound(false), exp.to_bound(true)) {
            (Some(a), Some(e)) => a > e,
            _ => false,
        },
        ComparisonOp::GreaterThanOrEqual => match (act.to_bound(false), exp.to_bound(false)) {
            (Some(a), Some(e)) => a >= e,
            _ => false,
        },
        ComparisonOp::LessThan => match (act.to_bound(true), exp.to_bound(false)) {
            (Some(a), Some(e)) => a < e,
            _ => false,
        },
        ComparisonOp::LessThanOrEqual => match (act.to_bound(true), exp.to_bound(true)) {
            (Some(a), Some(e)) => a <= e,
            _ => false,
        },
        ComparisonOp::StartsWith => actual.starts_with(expected),
        ComparisonOp::Contains => actual.contains(expected),
        ComparisonOp::EndsWith => actual.ends_with(expected),
        ComparisonOp::Regex => compare_string(actual, expected, ComparisonOp::Regex, false),
    }
}

pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    let mut t_chars = text.chars();
    let mut p_chars = pattern.chars();

    while let Some(p) = p_chars.next() {
        if p == '*' {
            let next_p = match p_chars.next() {
                Some(c) => c,
                None => return true,
            };
            loop {
                match t_chars.next() {
                    Some(t) if t == next_p => break,
                    None => return false,
                    _ => continue,
                }
            }
        } else if p == '?' {
            if t_chars.next().is_none() {
                return false;
            }
        } else {
            match t_chars.next() {
                Some(t) if t == p => continue,
                _ => return false,
            }
        }
    }

    t_chars.next().is_none()
}
