pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() > max_chars {
        s.chars().take(max_chars - 1).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}
