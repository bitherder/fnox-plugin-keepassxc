# AGENTS.md: fnox-plugin-keepassxc

Guidance for agents working in this repository. These rules are
mandatory. Follow them even when a shortcut looks faster.

## Layout

* `provider.lua` at the repo root is the fnox plugin entrypoint. It must
  stay there.
* `client/` is the Rust crate. `client/src/lib.rs` holds the logic, and
  `client/src/main.rs` stays thin.
* `spec/` holds the Lua tests. `spec/integration/` holds the tests that
  drive the real Rust binary.
* `doc/project-plan.md` is the living plan, decision log, and options hub.
  Record every decision, verified claim, and completed plan step there, as
  a dated log entry.

## Test-driven workflow (required)

Every behaviour change follows red, then green, and the two steps are
written by different agents.

1. **Red.** One subagent writes the failing tests and nothing else. It must
   not write production code. Run the tests and confirm they fail for the
   expected reason, such as a missing symbol or a wrong assertion, not a
   typo. Commit with a `#RED` tag at the end of the subject line:

   ```text
   test: expect exit code for locked database #RED
   ```

2. **Green.** A different agent writes the minimum code that makes the
   tests pass. "Different" means a separate subagent session, not the one
   that wrote the tests. The green agent must not edit the tests to make
   them pass. If a test looks wrong, stop and report it. Run the full
   suite, then commit with a `#GREEN` tag at the end of the subject line:

   ```text
   client: map locked database to exit code 4 #GREEN
   ```

Earlier commits use `#BROKEN` for the red step. Use `#RED` and `#GREEN`
from now on.

## Commit messages

* Subject: a lowercase component prefix, then an imperative description,
  under 50 characters, with no trailing period. The tag goes at the end of
  the subject.
* Body: optional. Explain why, not how. Wrap every line at 72 characters.
* Never amend or rewrite commits that already exist without explicit
  approval.
* Stage only the files that belong to the change, by name. Do not use
  `git add -A` or `git add .`.

## Markdown

* Before committing any change to a `.md` file, run `rumdl check <files>`
  and `rumdl fmt <files>` on the changed files. Fix anything that remains,
  and only then commit.

## Tests

* Rust: `cargo test --manifest-path client/Cargo.toml` and
  `cargo fmt --manifest-path client/Cargo.toml -- --check`.
* Lua: `./scripts/test-lua.sh`.
* Lua to Rust integration: `./scripts/test-integration.sh`.
* Everything: `mise run test`.

## Git remotes

* Do not push. Leave commits local, and let the user push. Do not run
  `git push`, even if asked to check whether a push succeeds.
