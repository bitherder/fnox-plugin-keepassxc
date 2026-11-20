//! Core logic for talking to KeePassXC's Browser Integration protocol.
//!
//! Kept separate from `main.rs` so the protocol/parsing logic is unit
//! testable without a running KeePassXC instance or CLI argument handling.

use keepassxc_client::KxcError;

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_AUTH_REQUIRED: i32 = 4;
pub const EXIT_UNAVAILABLE: i32 = 5;

pub fn exit_code_for(error: &KxcError) -> i32 {
    match error {
        KxcError::DatabaseLocked | KxcError::NotAssociated => EXIT_AUTH_REQUIRED,
        _ => EXIT_ERROR,
    }
}

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

    #[test]
    fn maps_locked_database_to_auth_required() {
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::DatabaseLocked),
            EXIT_AUTH_REQUIRED
        );
    }

    #[test]
    fn maps_unassociated_client_to_auth_required() {
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::NotAssociated),
            EXIT_AUTH_REQUIRED
        );
    }

    #[test]
    fn maps_missing_socket_to_unavailable() {
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::SocketNotFound("x".into())),
            EXIT_UNAVAILABLE
        );
    }

    #[test]
    fn maps_closed_connection_to_unavailable() {
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::ConnectionClosed),
            EXIT_UNAVAILABLE
        );
    }

    #[test]
    fn maps_io_failure_to_unavailable() {
        let io = std::io::Error::other("connection refused");
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::Io(io)),
            EXIT_UNAVAILABLE
        );
    }

    #[test]
    fn exit_codes_are_distinct_and_ok_is_zero() {
        assert_eq!(EXIT_OK, 0);
        let codes = [
            EXIT_OK,
            EXIT_ERROR,
            EXIT_USAGE,
            EXIT_AUTH_REQUIRED,
            EXIT_UNAVAILABLE,
        ];
        let unique: std::collections::HashSet<i32> = codes.into_iter().collect();
        assert_eq!(unique.len(), codes.len());
    }
}
