//! Persists the KeePassXC association under `$XDG_STATE_HOME`.
//!
//! The file holds a bearer credential (the identification key secret), so
//! it is written owner-only.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use keepassxc_client::crypto::b64;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

/// The KeePassXC association: the client id and the 32-byte secret that
/// proves possession of it. No `Debug` or `Clone`, so the secret cannot be
/// printed or copied by accident.
pub struct StoredAssociation {
    pub id: String,
    pub id_key_secret: [u8; 32],
}

/// Resolves the association file. A relative `xdg_state_home` is ignored,
/// as the XDG Base Directory spec requires.
pub fn state_path(xdg_state_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    let xdg = xdg_state_home
        .map(Path::new)
        .filter(|path| path.is_absolute());
    let state_dir = match xdg {
        Some(xdg) => xdg.to_path_buf(),
        None => home
            .map(Path::new)
            .filter(|path| path.is_absolute())?
            .join(".local/state"),
    };
    Some(
        state_dir
            .join("fnox-plugin-keepassxc")
            .join("association.json"),
    )
}

/// Returns `Ok(None)` when no association has been saved yet.
pub fn load(path: &Path) -> io::Result<Option<StoredAssociation>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let value: serde_json::Value = serde_json::from_str(&text).map_err(invalid_data)?;
    let id = value["id"]
        .as_str()
        .ok_or_else(|| invalid_data("missing id"))?;
    let secret = value["id_key_secret"]
        .as_str()
        .ok_or_else(|| invalid_data("missing id_key_secret"))?;
    let id_key_secret = b64::dec(secret)
        .map_err(invalid_data)?
        .try_into()
        .map_err(|_| invalid_data("id_key_secret is not 32 bytes"))?;
    Ok(Some(StoredAssociation {
        id: id.to_string(),
        id_key_secret,
    }))
}

/// Writes the file and its directory owner-only (0600 and 0700 on Unix),
/// tightening an existing file or directory that is looser.
pub fn save(path: &Path, assoc: &StoredAssociation) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
        #[cfg(unix)]
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let json = serde_json::json!({
        "id": assoc.id,
        "id_key_secret": b64::enc(&assoc.id_key_secret),
    });
    write_private(path, json.to_string().as_bytes())
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use io::Write;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)
}

fn invalid_data<E>(error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}

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
    fn data_path_uses_xdg_data_home_when_absolute() {
        assert_eq!(
            data_path(Some(OsStr::new("/xdg/data")), Some(OsStr::new("/home/me"))),
            Some(PathBuf::from(
                "/xdg/data/fnox-plugin-keepassxc/association.json"
            ))
        );
    }

    #[test]
    fn data_path_ignores_relative_xdg_data_home() {
        assert_eq!(
            data_path(
                Some(OsStr::new("relative/data")),
                Some(OsStr::new("/home/me"))
            ),
            Some(PathBuf::from(
                "/home/me/.local/share/fnox-plugin-keepassxc/association.json"
            ))
        );
    }

    #[test]
    fn data_path_falls_back_to_home_local_share() {
        assert_eq!(
            data_path(None, Some(OsStr::new("/home/me"))),
            Some(PathBuf::from(
                "/home/me/.local/share/fnox-plugin-keepassxc/association.json"
            ))
        );
    }

    #[test]
    fn data_path_is_none_without_xdg_or_home() {
        assert_eq!(data_path(None, None), None);
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
