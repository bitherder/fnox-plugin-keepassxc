use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kpxc-client");

/// A per-test directory that is wiped on creation and again on drop, so a
/// failing assertion still cleans up. Every child process gets its own
/// HOME, XDG and TMPDIR here, so the developer's real config cannot leak in.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(test: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("fnox-cli-test-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in [root.join("state"), root.join("runtime"), root.join("tmp")] {
            fs::create_dir_all(dir).unwrap();
        }
        Self { root }
    }

    fn state_dir(&self) -> PathBuf {
        self.root.join("state")
    }

    fn association_file(&self) -> PathBuf {
        self.state_dir()
            .join("fnox-plugin-keepassxc")
            .join("association.json")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .env("HOME", &self.root)
            .env("XDG_STATE_HOME", self.state_dir())
            .env("XDG_RUNTIME_DIR", self.root.join("runtime"))
            .env("TMPDIR", self.root.join("tmp"))
            .output()
            .unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn version_flag_prints_version_and_exits_zero() {
    let sandbox = Sandbox::new("version_flag_prints_version_and_exits_zero");

    let output = sandbox.run(&["--version"]);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).starts_with("kpxc-client "),
        "stdout: {}",
        stdout(&output)
    );
}

#[test]
fn reference_without_association_exits_not_associated() {
    let sandbox = Sandbox::new("reference_without_association_exits_not_associated");

    let output = sandbox.run(&["fnox://github-token"]);

    assert_eq!(output.status.code(), Some(4), "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("associate"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn reference_with_stored_association_but_no_keepassxc_exits_unavailable() {
    let sandbox =
        Sandbox::new("reference_with_stored_association_but_no_keepassxc_exits_unavailable");
    fs::create_dir_all(sandbox.association_file().parent().unwrap()).unwrap();
    fs::write(
        sandbox.association_file(),
        r#"{"id":"fnox-test","id_key_secret":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}"#,
    )
    .unwrap();

    let output = sandbox.run(&["fnox://github-token"]);

    assert_eq!(output.status.code(), Some(5), "stderr: {}", stderr(&output));
}

#[test]
fn associate_without_keepassxc_exits_unavailable() {
    let sandbox = Sandbox::new("associate_without_keepassxc_exits_unavailable");

    let output = sandbox.run(&["associate"]);

    assert_eq!(output.status.code(), Some(5), "stderr: {}", stderr(&output));
}
