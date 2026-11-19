//! Core logic for talking to KeePassXC's Browser Integration protocol.
//!
//! Kept separate from `main.rs` so the protocol/parsing logic is unit
//! testable without a running KeePassXC instance or CLI argument handling.

/// Placeholder: parses an `fnox://<name>` reference into the bare name used
/// to tag entries' URL fields in KeePassXC.
///
/// Returns `None` if `reference` doesn't use the expected scheme.
pub fn parse_reference(reference: &str) -> Option<&str> {
    reference.strip_prefix("fnox://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_reference() {
        assert_eq!(parse_reference("fnox://github-token"), Some("github-token"));
    }

    #[test]
    fn rejects_a_reference_without_the_scheme() {
        assert_eq!(parse_reference("github-token"), None);
    }

    #[test]
    fn rejects_an_empty_string() {
        assert_eq!(parse_reference(""), None);
    }
}
