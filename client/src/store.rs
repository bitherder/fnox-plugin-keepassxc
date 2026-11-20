//! Persists the KeePassXC association under `$XDG_STATE_HOME`.
//!
//! The file holds a bearer credential (the identification key secret), so
//! it is written owner-only.

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn fresh_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fnox-store-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn state_path_uses_xdg_state_home_when_absolute() {
        assert_eq!(
            state_path(Some(OsStr::new("/xdg/state")), Some(OsStr::new("/home/me"))),
            Some(PathBuf::from(
                "/xdg/state/fnox-plugin-keepassxc/association.json"
            ))
        );
    }

    #[test]
    fn state_path_ignores_relative_xdg_state_home() {
        assert_eq!(
            state_path(
                Some(OsStr::new("relative/state")),
                Some(OsStr::new("/home/me"))
            ),
            Some(PathBuf::from(
                "/home/me/.local/state/fnox-plugin-keepassxc/association.json"
            ))
        );
    }

    #[test]
    fn state_path_falls_back_to_home_local_state() {
        assert_eq!(
            state_path(None, Some(OsStr::new("/home/me"))),
            Some(PathBuf::from(
                "/home/me/.local/state/fnox-plugin-keepassxc/association.json"
            ))
        );
    }

    #[test]
    fn state_path_is_none_without_xdg_or_home() {
        assert_eq!(state_path(None, None), None);
    }

    #[test]
    fn save_then_load_round_trips_id_and_secret() {
        let dir = fresh_dir("round_trip");
        let path = dir.join("state").join("association.json");
        let secret: [u8; 32] = std::array::from_fn(|i| i as u8);
        let assoc = StoredAssociation {
            id: "fnox-client".to_string(),
            id_key_secret: secret,
        };

        save(&path, &assoc).unwrap();
        let loaded = load(&path).unwrap().unwrap();

        assert_eq!(loaded.id, "fnox-client");
        assert_eq!(loaded.id_key_secret, secret);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_returns_none_when_file_is_missing() {
        let dir = fresh_dir("missing");
        let path = dir.join("association.json");

        assert!(load(&path).unwrap().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_rejects_malformed_json() {
        let dir = fresh_dir("malformed");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("association.json");
        fs::write(&path, b"{not json").unwrap();

        assert!(load(&path).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_rejects_a_secret_that_is_not_32_bytes() {
        let dir = fresh_dir("wrong_length");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("association.json");

        // 16 zero bytes, then 33 zero bytes, both valid standard base64.
        for secret_b64 in [
            "AAAAAAAAAAAAAAAAAAAAAA==",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            let json = format!(r#"{{"id": "fnox-client", "id_key_secret": "{secret_b64}"}}"#);
            fs::write(&path, json).unwrap();

            assert!(load(&path).is_err(), "accepted secret {secret_b64}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn save_writes_owner_only_file_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = fresh_dir("file_mode");
        let path = dir.join("association.json");
        let assoc = StoredAssociation {
            id: "fnox-client".to_string(),
            id_key_secret: [9u8; 32],
        };

        save(&path, &assoc).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();

        assert_eq!(mode & 0o777, 0o600);
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn save_creates_directory_with_owner_only_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = fresh_dir("dir_mode");
        let parent = dir.join("state");
        let path = parent.join("association.json");
        let assoc = StoredAssociation {
            id: "fnox-client".to_string(),
            id_key_secret: [9u8; 32],
        };

        save(&path, &assoc).unwrap();
        let mode = fs::metadata(&parent).unwrap().permissions().mode();

        assert_eq!(mode & 0o777, 0o700);
        let _ = fs::remove_dir_all(&dir);
    }
}
