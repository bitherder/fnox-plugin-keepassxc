//! Command-line dispatch: `--version`, `associate`, and `fnox://<name>`.
//!
//! The binary is a thin wrapper around `run`, so the argument handling and
//! exit-code mapping stay testable as a library.

use std::path::PathBuf;

use keepassxc_client::{IdKeyPair, KxcClient, KxcError, transport::unix::UnixTransport};

use crate::store::{self, StoredAssociation};
use crate::{
    EXIT_AUTH_REQUIRED, EXIT_ERROR, EXIT_OK, EXIT_USAGE, Lookup, exit_code_for,
    exit_code_for_lookup, lookup, parse_reference,
};

const USAGE: &str = "usage: kpxc-client --version | associate | <fnox://name>";

pub async fn run(args: &[String]) -> i32 {
    match args {
        [arg] if arg == "--version" => {
            println!("kpxc-client {}", env!("CARGO_PKG_VERSION"));
            EXIT_OK
        }
        [arg] if arg == "associate" => associate().await,
        [reference] => match parse_reference(reference) {
            Some(name) => fetch(name).await,
            None => {
                eprintln!("invalid reference: {reference} (expected fnox://<name>)");
                EXIT_USAGE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            EXIT_USAGE
        }
    }
}

async fn associate() -> i32 {
    let path = match data_path() {
        Ok(path) => path,
        Err(code) => return code,
    };
    let transport = match UnixTransport::connect_auto().await {
        Ok(transport) => transport,
        Err(error) => return report(&error),
    };
    let mut client = KxcClient::new(transport);
    let key = IdKeyPair::generate();
    if let Err(error) = client.change_public_keys().await {
        return report(&error);
    }
    let association = match client.associate(&key).await {
        Ok(association) => association,
        Err(error) => return report(&error),
    };
    let stored = StoredAssociation {
        id: association.id,
        id_key_secret: key.secret_bytes(),
    };
    if let Err(error) = store::save(&path, &stored) {
        eprintln!("could not save association to {}: {error}", path.display());
        return EXIT_ERROR;
    }
    println!("associated with KeePassXC; saved to {}", path.display());
    EXIT_OK
}

async fn fetch(name: &str) -> i32 {
    let path = match data_path() {
        Ok(path) => path,
        Err(code) => return code,
    };
    let association = match store::load(&path) {
        Ok(Some(association)) => association,
        Ok(None) => {
            eprintln!("not associated: run `kpxc-client associate` with KeePassXC unlocked");
            return EXIT_AUTH_REQUIRED;
        }
        Err(error) => {
            eprintln!(
                "could not read association from {}: {error}",
                path.display()
            );
            return EXIT_ERROR;
        }
    };
    let transport = match UnixTransport::connect_auto().await {
        Ok(transport) => transport,
        Err(error) => return report(&error),
    };
    let mut client = KxcClient::new(transport);
    let key = IdKeyPair::from_secret_bytes(association.id_key_secret);
    if let Err(error) = client.change_public_keys().await {
        return report(&error);
    }
    if let Err(error) = client.test_associate(&association.id, &key).await {
        return report(&error);
    }
    let outcome = match lookup(&mut client, name).await {
        Ok(outcome) => outcome,
        Err(error) => return report(&error),
    };
    match &outcome {
        Lookup::Found(password) => println!("{password}"),
        Lookup::NotFound => eprintln!("no entry found for fnox://{name}"),
        Lookup::Ambiguous(count) => {
            eprintln!("several entries match fnox://{name} ({count} found)")
        }
    }
    exit_code_for_lookup(&outcome)
}

fn data_path() -> Result<PathBuf, i32> {
    let xdg = std::env::var_os("XDG_DATA_HOME");
    let home = std::env::var_os("HOME");
    store::data_path(xdg.as_deref(), home.as_deref()).ok_or_else(|| {
        eprintln!("cannot locate data directory: set HOME or XDG_DATA_HOME");
        EXIT_ERROR
    })
}

fn report(error: &KxcError) -> i32 {
    eprintln!("kpxc-client: {error}");
    exit_code_for(error)
}
