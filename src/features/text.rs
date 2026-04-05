pub fn slugify(input: &str) -> String {
    let lowered = input.to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_dash = false;

    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

pub fn excerpt(input: &str, max_len: usize) -> String {
    if input.len() <= max_len {
        return input.to_string();
    }
    let mut s = input[..max_len].to_string();
    if let Some(last_space) = s.rfind(' ') {
        s.truncate(last_space);
    }
    s.push_str("...");
    s
}
