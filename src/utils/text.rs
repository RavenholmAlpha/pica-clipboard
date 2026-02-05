use regex::Regex;
use std::sync::OnceLock;

static SENSITIVE_REGEX: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_sensitive_patterns() -> &'static Vec<Regex> {
    SENSITIVE_REGEX.get_or_init(|| {
        vec![
            Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            Regex::new(r"-----BEGIN PRIVATE KEY-----").unwrap(),
        ]
    })
}

pub fn is_sensitive(text: &str) -> bool {
    let patterns = get_sensitive_patterns();
    for pattern in patterns {
        if pattern.is_match(text) {
            return true;
        }
    }
    false
}

pub fn clean_text(text: &str) -> String {
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sensitive() {
        assert!(is_sensitive("Here is my key: sk-abcdef1234567890abcdef123456"));
        assert!(!is_sensitive("Hello world"));
    }

    #[test]
    fn test_clean() {
        assert_eq!(clean_text("  hello  "), "hello");
    }
}
