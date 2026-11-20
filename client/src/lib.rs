//! Core logic for talking to KeePassXC's Browser Integration protocol.
//!
//! Kept separate from `main.rs` so the protocol/parsing logic is unit
//! testable without a running KeePassXC instance or CLI argument handling.

use keepassxc_client::{KxcClient, KxcError, transport::Transport};

mod store;

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_AUTH_REQUIRED: i32 = 4;
pub const EXIT_UNAVAILABLE: i32 = 5;

pub fn exit_code_for(error: &KxcError) -> i32 {
    match error {
        KxcError::DatabaseLocked | KxcError::NotAssociated => EXIT_AUTH_REQUIRED,
        KxcError::SocketNotFound(_) | KxcError::ConnectionClosed | KxcError::Io(_) => {
            EXIT_UNAVAILABLE
        }
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

#[derive(Debug, PartialEq, Eq)]
pub enum Lookup {
    Found(String),
    NotFound,
    Ambiguous(usize),
}

pub async fn lookup<T: Transport>(
    client: &mut KxcClient<T>,
    name: &str,
) -> Result<Lookup, KxcError> {
    let url = format!("fnox://{name}");
    let entries = client.get_logins(&url).await?;
    Ok(match entries.as_slice() {
        [] => Lookup::NotFound,
        [entry] => Lookup::Found(entry.password.clone()),
        many => Lookup::Ambiguous(many.len()),
    })
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
    fn maps_crypto_failure_to_error() {
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::Crypto),
            EXIT_ERROR
        );
    }

    #[test]
    fn maps_nonce_mismatch_to_error() {
        assert_eq!(
            exit_code_for(&keepassxc_client::KxcError::NonceMismatch),
            EXIT_ERROR
        );
    }

    #[test]
    fn maps_keepassxc_error_to_error() {
        let error = keepassxc_client::KxcError::KeePassXc {
            code: Some(1),
            message: "denied".into(),
        };
        assert_eq!(exit_code_for(&error), EXIT_ERROR);
    }

    #[test]
    fn maps_unexpected_response_to_error() {
        let error = keepassxc_client::KxcError::UnexpectedResponse {
            action: "get-logins".into(),
            detail: "missing entries".into(),
        };
        assert_eq!(exit_code_for(&error), EXIT_ERROR);
    }

    #[test]
    fn maps_found_lookup_to_ok() {
        assert_eq!(exit_code_for_lookup(&Lookup::Found("x".into())), EXIT_OK);
    }

    #[test]
    fn maps_missing_entry_to_not_found() {
        assert_eq!(exit_code_for_lookup(&Lookup::NotFound), EXIT_NOT_FOUND);
    }

    #[test]
    fn maps_ambiguous_lookup_to_error() {
        assert_eq!(exit_code_for_lookup(&Lookup::Ambiguous(2)), EXIT_ERROR);
    }

    #[test]
    fn exit_codes_are_distinct_and_ok_is_zero() {
        assert_eq!(EXIT_OK, 0);
        let codes = [
            EXIT_OK,
            EXIT_ERROR,
            EXIT_USAGE,
            EXIT_NOT_FOUND,
            EXIT_AUTH_REQUIRED,
            EXIT_UNAVAILABLE,
        ];
        let unique: std::collections::HashSet<i32> = codes.into_iter().collect();
        assert_eq!(unique.len(), codes.len());
    }
}
