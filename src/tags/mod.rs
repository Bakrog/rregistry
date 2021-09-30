use regex::Regex;

/// Validate tag names using the regex `^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$`
pub fn is_tag_name_valid(name: &str) -> bool {
    let regex = Regex::new(r"^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$").unwrap();
    regex.is_match(name)
}

/// Validate digest using the regex `^[a-z0-9]+([+._-][a-z0-9]+)*:[a-zA-Z0-9=_-]+$`
pub fn is_accepted_digest(digest: &str) -> bool {
    let regex = Regex::new(r"^[a-z0-9]+([+._-][a-z0-9]+)*:[a-zA-Z0-9=_-]+$").unwrap();
    regex.is_match(digest)
}
