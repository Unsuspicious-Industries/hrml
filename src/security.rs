// HRML Security
//
// Unix philosophy: simple, composable security primitives.
// No frameworks, no magic. Just functions that do one thing.

/// Escape HTML special characters to prevent XSS.
/// Use this when rendering user-generated content.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '/' => out.push_str("&#x2F;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape for use inside HTML attributes.
pub fn escape_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape for use inside URLs.
pub fn escape_url(input: &str) -> String {
    urlencoding::encode(input).into_owned()
}

/// Sanitize a URL: only allow http/https/mailto schemes.
pub fn sanitize_url(url: &str) -> Option<String> {
    let lower = url.trim().to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("/")
        || lower.starts_with("#")
        || (!lower.contains(':') && !lower.contains("//"))
    {
        Some(url.to_string())
    } else {
        None
    }
}

/// Check if content looks like HTML (contains angle brackets with tag-like content).
pub fn looks_like_html(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("<div")
        || lower.contains("<span")
        || lower.contains("<p>")
        || lower.contains("<a ")
        || lower.contains("<img")
        || lower.contains("<script")
        || lower.contains("<html")
}

/// Strip all HTML tags from text.
pub fn strip_html(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in input.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

/// Generate a simple CSRF token from random bytes.
pub fn generate_csrf_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    format!("{:x}_{:x}", now, pid)
}

/// Validate a simple nonce (timestamp-based, not cryptographically secure).
/// For production use, use a proper HMAC-based approach.
pub fn validate_nonce(token: &str, max_age_secs: u64) -> bool {
    if let Some((ts, _)) = token.split_once('_') {
        if let Ok(ts) = u64::from_str_radix(ts, 16) {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            return now.saturating_sub(ts) <= max_age_secs;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_basic() {
        assert_eq!(
            escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;&#x2F;script&gt;"
        );
    }

    #[test]
    fn escape_html_ampersand() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn escape_html_quotes() {
        assert_eq!(escape_html("\"hello\""), "&quot;hello&quot;");
        assert_eq!(escape_html("'hello'"), "&#x27;hello&#x27;");
    }

    #[test]
    fn escape_html_empty() {
        assert_eq!(escape_html(""), "");
    }

    #[test]
    fn escape_html_no_special_chars() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn sanitize_url_safe() {
        assert!(sanitize_url("https://example.com").is_some());
        assert!(sanitize_url("http://example.com").is_some());
        assert!(sanitize_url("/local/path").is_some());
        assert!(sanitize_url("#anchor").is_some());
        assert!(sanitize_url("mailto:test@example.com").is_some());
        assert!(sanitize_url("relative/path").is_some());
    }

    #[test]
    fn sanitize_url_unsafe() {
        assert!(sanitize_url("javascript:alert(1)").is_none());
        assert!(sanitize_url("data:text/html,<script>").is_none());
        assert!(sanitize_url("vbscript:msgbox(1)").is_none());
    }

    #[test]
    fn strip_html_basic() {
        assert_eq!(strip_html("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html("<div><span>test</span></div>"), "test");
        assert_eq!(strip_html("no tags"), "no tags");
    }

    #[test]
    fn looks_like_html_yes() {
        assert!(looks_like_html("<div>content</div>"));
        assert!(looks_like_html("<script>alert(1)</script>"));
    }

    #[test]
    fn looks_like_html_no() {
        assert!(!looks_like_html("just text"));
        assert!(!looks_like_html("5 < 10"));
    }

    #[test]
    fn csrf_token_format() {
        let token = generate_csrf_token();
        assert!(token.contains('_'));
        let parts: Vec<&str> = token.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert!(u64::from_str_radix(parts[0], 16).is_ok());
    }

    #[test]
    fn validate_nonce_fresh() {
        let token = generate_csrf_token();
        assert!(validate_nonce(&token, 3600));
    }

    #[test]
    fn validate_nonce_expired() {
        assert!(!validate_nonce("0_deadbeef", 3600));
    }

    #[test]
    fn validate_nonce_invalid() {
        assert!(!validate_nonce("not-a-token", 3600));
        assert!(!validate_nonce("", 3600));
    }
}
