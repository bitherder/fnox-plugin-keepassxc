# fnox-plugin-keepassxc: project plan, log, and options hub

Living document. Update it whenever a decision is made, a question is resolved, or a claim
is verified. Add dated entries to the [log](#log); do not rewrite history. If a past claim
turns out to be wrong, record the correction in the log and fix the claim where it appears.

Status key used throughout: **Verified** (observed or read from source), **Documented**
(taken from upstream docs, not tested here), **Unverified** (assumption or recollection).

## Goal

Expose authentication tokens stored in KeePassXC to programs as environment variables, using
[fnox](https://fnox.jdx.dev) for secret resolution and [mise](https://mise.jdx.dev) for tool
management, **without decrypting the whole database in the calling process**.

Starting scope: read-only retrieval of individual tokens via a running, unlocked KeePassXC.
Writing secrets, TOTP, and headless/CI use are out of scope until the read path works.

## Readiness checklist (live use)

Goal: remove the tokens from `mise.toml` and read them through fnox. `[x]` is done; `[ ]` is
open. Items marked **(you)** need your action in a live KeePassXC session. Details are in the
sections named in brackets.

Built:

* [x] Protocol client: key exchange, association, `test-associate`, `get-logins` (pinned
  `keepassxc-client` 0.1.0) [Option B]
* [x] Exit-code contract shared by the Rust client and `provider.lua` [Tests that exist]
* [x] Association stored as a `0600` file under `$XDG_DATA_HOME` [Option D]
* [x] CLI: `--version`, `associate`, and `fnox://<name>` [Current code state]

Verify live:

* [x] **(you)** Run `kpxc-client associate` with KeePassXC unlocked, and approve the prompt
* [x] **(you)** Run `kpxc-client fnox://<name>` for a test entry, then for a real token entry
* [x] Run `fnox get KPXC_TEST_PRIMARY`; user reports it returned the test-entry value
* [x] Repeat with absolute `XDG_DATA_HOME`; fnox ran `auth_command`, associated the test DB, and
  retried the lookup successfully
* [ ] **(you)** Tag each token entry in KeePassXC with a `fnox://<name>` URL [Option C]

Wire up:

* [x] Add a `build` mise task (`mise run build`) that builds `client/target/release/kpxc-client`
* [x] Implement plugin-relative lookup and verify it using real fnox [Binary location and wiring]
* [x] Run `fnox provider test` with the local plugin and release binary [Lua plugins]
* [x] Verify mise task injection with a nonempty check and no value output
* [x] Verify warm-cache behavior: with `FNOX_DAEMON=on`, a repeated task avoids KeePassXC prompt;
  with it off, KeePassXC prompts again
* [ ] Test a real application consuming a secret through mise or `fnox exec`
* [ ] Choose the plugin source for development (`fnox plugin link`) and, later, a git source
* [x] Document `fnox.toml` provider and secret-reference examples [Binary location and wiring]
* [ ] Add a worked `mise.toml` task using `[secrets.fnox]`
* [x] Choose task-scoped mise secrets for mise tasks; the CLI fallback uses fnox's daemon cache
  [D13]
* [ ] Decide whether non-mise commands also need `fnox exec` [Known risks, item 9]
* [x] Choose security stance: keep KeePassXC **Remember** unticked; use fnox cache for warm reads
  [Known risks, item 4]
* [x] Wire and verify `auth_command` to `kpxc-client associate` [Lua plugins]

Finish:

* [ ] Remove the tokens from `mise.toml` once the checks above pass
* [x] Record reported live associate, CLI lookup, `fnox get`, and mise task results without logging
  any secret values.

## Why not the built-in fnox `keepass` provider

**Documented** ([fnox keepass provider](https://fnox.jdx.dev/providers/keepass.html)):

* It opens the `.kdbx` file directly. KeePassXC is not involved at all.
* It needs the master password in `FNOX_KEEPASS_PASSWORD` / `KEEPASS_PASSWORD`, or a prompt,
  and decrypts the entire database in the fnox process.
* It cannot borrow an unlock performed in the KeePassXC GUI (Touch ID, YubiKey, and so on).

That is the discomfort this project exists to avoid.

## Decisions

| #   | Decision                                                                                                                                 | Date       | Notes                                                                                                                      |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------- |
| D1  | Integrate with KeePassXC through the **Browser Integration protocol** first; do not move to other approaches without consulting the user | 2026-11-19 | User instruction. See [options](#option-a-keepassxc-access-mechanism)                                                      |
| D2  | Implement as an **fnox Lua plugin** plus a **Rust client binary**                                                                        | 2026-11-19 | Lua cannot do sockets or NaCl crypto, so a helper binary is unavoidable                                                    |
| D3  | Repo at `~/Projects/fnox-plugin-keepassxc`; fnox's own term is "provider", and shareable plugins are conventionally `fnox-plugin-<name>` | 2026-11-19 |                                                                                                                            |
| D4  | Layout: `provider.lua` and `spec/` at repo root, Rust crate in `client/`                                                                 | 2026-11-19 | `provider.lua` must be at the root for `owner/repo` git installs                                                           |
| D5  | Name the Rust piece **`client`** (not `bridge`, `wrapper`, `core`, `agent`)                                                              | 2026-11-19 | User choice; it is a protocol client                                                                                       |
| D6  | No `lib/` directory until Lua helpers are actually needed                                                                                | 2026-11-19 |                                                                                                                            |
| D7  | Automated tests first, for both Lua and Rust                                                                                             | 2026-11-19 | `mise run test` runs unit, Rust, and integration suites                                                                    |
| D8  | Rust for the client                                                                                                                      | 2026-11-19 | Single static binary, installable via mise                                                                                 |
| D9  | Do not re-implement the protocol; use an existing crate                                                                                  | 2026-11-20 | Chose `keepassxc-client` (D10)                                                                                             |
| D10 | Use `keepassxc-client` (KC) as the protocol base, pinned exactly (`=0.1.0`)                                                              | 2026-11-20 | User decision. Supersedes the "behind a trait of our own" plan. See [Option B](#option-b-protocol-library)                 |
| D11 | Keep a Rust binary, but make it a thin CLI over KC; it owns no protocol code                                                             | 2026-11-20 | Lua has no sockets or NaCl crypto, and fnox can only run processes. See [Re-evaluation](#re-evaluation-2026-11-20)         |
| D12 | Find the client in order: `client_bin`, plugin-local release binary, then PATH. Build locally with mise; defer release binaries          | 2026-11-20 | Implemented and verified with fnox 1.50.0; see [Binary location and wiring](#binary-location-and-wiring)                   |
| D13 | Use task-scoped mise secrets with fnox CLI fallback and fnox daemon cache; direct socket optimization is not required                    | 2026-11-24 | User choice after on/off prompt test. Keep fnox 1.50.0; do not downgrade for mise 2026.10.17. Actual app use remains open. |

## Architecture (current intent)

```text
fnox ──(Lua hook get(ctx, ref))──> provider.lua
                                      │  fnox.cmd.run({ client_bin, ref })
                                      ▼
                                 kpxc-client  (Rust, one process per lookup)
                                      │  Unix socket, NaCl-box encrypted
                                      ▼
                         running, unlocked KeePassXC
```

* `provider.lua` is thin: run the client, map outcomes to fnox semantics. Return `nil` for
  "not found" (so fnox `default`/`if_missing` apply), `fnox.auth_error(...)` when the
  database is locked or not associated (fnox can then offer `auth_command`), and `error(...)`
  for everything else.
* `client/src/lib.rs` holds testable logic; `main.rs` is a thin CLI.
* KC's `KxcClient<T: Transport>` is already generic over its transport, so lookup logic can
  be tested in-process with KC's public `transport::mock::MockTransport`. No extra trait of
  our own is needed unless we later swap libraries.
* `get_batch` can later resolve several secrets in one process and one connection.

### Binary location and wiring

`provider.lua` resolves the client binary in this order:

1. The `client_bin` provider option, if set.
2. `<ctx.plugin_dir>/client/target/release/kpxc-client`, if the file exists.
3. `kpxc-client` on PATH.

`mise run build` writes the release binary to `client/target/release/`. A git-installed plugin has
no build step, so users must run `mise run build` in the cloned plugin directory. After that,
`provider.lua` finds it through `ctx.plugin_dir`; no global install or PATH entry is needed. The
`auth_command` is run outside the plugin and still needs an explicit path or a binary on PATH.

**Verified:** on 2026-11-23, fnox 1.50.0 ran `fnox provider test` with the repository as a local
plugin and found its release binary through `ctx.plugin_dir`. No `client_bin` option or PATH entry
was set.

Development, from a linked checkout:

```toml
# fnox.toml
[providers.keepassxc]
type = "lua"
plugin = "keepassxc"   # the name given to `fnox plugin link`
auth_command = "<checkout>/client/target/release/kpxc-client associate"

[secrets]
GITHUB_TOKEN = { provider = "keepassxc", value = "fnox://github-token" }
```

Run `fnox plugin link keepassxc ~/Projects/fnox-plugin-keepassxc` once. Then
`fnox provider test keepassxc` checks that the plugin loads.

Sharing, from a git source (build the binary in the plugin clone with `mise run build`):

```toml
[plugins]
keepassxc = "bitherder/fnox-plugin-keepassxc#v0.1.0"

[providers.keepassxc]
type = "lua"
plugin = "keepassxc"
```

The plugin-relative lookup finds the built client. `auth_command` runs outside the plugin, so it
still needs an absolute path or a binary on PATH.

### Current code state

The client has `parse_reference`, the exit-code mapping (`exit_code_for` and
`exit_code_for_lookup`), and `lookup`, which resolves a name against KeePassXC through
`keepassxc-client`. The `lookup` tests run against an in-process fake. The CLI in `cli.rs` supports
`--version`, `associate`, and `fnox://<name>`, and it reads the stored association. `provider.lua`
maps exit codes. No protocol code is written here; it comes from the library. The user reports
running `associate` and lookups for a test entry and a real token against live KeePassXC on
2026-11-23; exact output was not captured. `fnox provider test` with a local plugin path is
verified. The user also reports that `fnox get KPXC_TEST_PRIMARY` returned the test-entry value
after a fresh association through `auth_command`. A task-scoped mise secret test verified that the
value reaches a process using only a nonempty assertion. A real application consumer and
`fnox exec` remain untested.

### Tests that exist

| Suite                   | Command                                        | What it covers                               | Status after KC decision                                    |
| ----------------------- | ---------------------------------------------- | -------------------------------------------- | ----------------------------------------------------------- |
| Lua unit (busted)       | `./scripts/test-lua.sh`                        | `provider.get` against a mocked `fnox.cmd`   | Covers success and exit-code mapping                        |
| Rust unit               | `cargo test --manifest-path client/Cargo.toml` | parsing, error mapping, CLI and lookup logic | Passing; includes store and lookup tests                    |
| Lua-to-Rust integration | `./scripts/test-integration.sh`                | real binary without a KeePassXC dependency   | Covers invalid reference and missing-association auth error |

See [Re-evaluation](#re-evaluation-2026-11-20) for the proposed target structure.

## What fnox gives a plugin

**Documented** ([Lua plugins](https://fnox.jdx.dev/providers/lua.html)):

* Hooks: `get(ctx, ref)` (required), `get_batch`, `put`, `encrypt`, `test`.
* Modules: `fnox`, `fnox.http`, `fnox.json`, `fnox.cmd`, `fnox.log`, `fnox.base64`. There is
  no socket access and no NaCl crypto, hence the external client binary.
* `fnox.auth_error(msg)` plus a provider-level `auth_command` lets fnox offer to run a command
  (a natural home for `kpxc-client associate`).
* Plugin state does not persist between lookups (fnox may reload the plugin), so any caching
  must live in the client or fnox, not in Lua module variables.
* Providers and secrets accept `daemon_cache`; it defaults to true and can opt a value out of the
  fnox daemon's memory cache. The daemon itself is optional and disabled by default. With it
  enabled, warm cache reads can avoid calling `provider.get`; cache misses resolve through the
  provider. Verified behavior for this plugin is in
  [Known risks and open questions](#known-risks-and-open-questions).

## Options

### Option A: KeePassXC access mechanism

|                                                  | Browser Integration protocol (chosen first, D1)                            | `keepassxc-cli`                        |
| ------------------------------------------------ | -------------------------------------------------------------------------- | -------------------------------------- |
| Talks to                                         | The running, unlocked KeePassXC app over a local socket                    | The `.kdbx` file directly              |
| Master password handled by us                    | No                                                                         | Yes (arg, stdin, or env)               |
| Reuses GUI unlock (Touch ID, YubiKey)            | Yes                                                                        | No                                     |
| Exposure boundary                                | Only entries matched by `get-logins`, plus KeePassXC's confirmation dialog | Everything the credentials can decrypt |
| Works headless or in CI                          | No (needs a running, unlocked app)                                         | Yes                                    |
| Engineering cost                                 | Higher: key exchange, NaCl box, association                                | Low: wrap the CLI                      |
| Same shape as fnox's built-in `keepass` provider | No                                                                         | Effectively yes                        |

Assessment: the CLI does not address the original concern. It is the same file-decrypting
design as the built-in provider. The protocol is the better fit for interactive use. If
CI or headless use becomes a requirement, the likely answer is a second provider (the
built-in `keepass` against a CI-scoped database), not the CLI. **Do not move to the CLI
without consulting the user (D1).**

Protocol references: the
[protocol spec](https://github.com/keepassxreboot/keepassxc-browser/blob/develop/keepassxc-protocol.md)
and the [browser extension](https://github.com/keepassxreboot/keepassxc-browser).

### Option B: protocol library

Both crates were downloaded and read in full on 2026-11-20. At comparison time neither had been
run against KeePassXC. Later live tests validated `keepassxc-client`; `keepassxc-proxy-lib` was not
used.

|                                        | `keepassxc-proxy-lib` 0.2.0                                                                              | `keepassxc-client` 0.1.0                                                                                                             |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Repo                                   | git.dog/xx/keepassxc-proxy-lib                                                                           | gitlab.com/GaffaFix/keepassxc-client                                                                                                 |
| Licence                                | MIT                                                                                                      | MIT OR Apache-2.0                                                                                                                    |
| Model                                  | Blocking, std only (about 900 lines)                                                                     | Async on tokio (about 950 lines)                                                                                                     |
| Socket read                            | Reads until a short read (<4096 bytes); can truncate a split reply or block on an exact multiple of 4096 | Incremental JSON stream parsing                                                                                                      |
| Signals (`database-locked`/`unlocked`) | Not handled; surfaces as "No nonce in response"                                                          | Skipped and queued                                                                                                                   |
| Reply nonce (+1) check                 | None                                                                                                     | Yes, with a tamper test                                                                                                              |
| "No logins found" (code 15)            | Error (`KeePassXCErrorCode`), mappable by us                                                             | Normalized to an empty vec                                                                                                           |
| Custom `KPH:` fields                   | `string_fields`                                                                                          | `string_fields`                                                                                                                      |
| `triggerUnlock`                        | Yes (`test_associate(true)`)                                                                             | Not exposed                                                                                                                          |
| Association                            | `{name, public_key}` (serde)                                                                             | `{id, idKey}` and `IdKeyPair`                                                                                                        |
| Own tests                              | Three trivial tests                                                                                      | Fake-KeePassXC round trips (private to the crate)                                                                                    |
| macOS                                  | "untested"                                                                                               | Socket candidates include `$TMPDIR`; upstream validation was Linux-only. This project later live-tested `keepassxc-client` on macOS. |
| Maturity                               | 3 releases, about 360 downloads, one maintainer                                                          | 1 release, about 18 downloads, one maintainer                                                                                        |
| Windows                                | Named pipe                                                                                               | Stub (irrelevant here)                                                                                                               |

Trade-off summary:

* `proxy-lib` is the better shape for a one-shot CLI but its receive path is not reliable in
  edge cases, and those are private functions we cannot patch from outside. We would need an
  upstream PR, a fork, or vendoring.
* `keepassxc-client` handles those edge cases but brings tokio and is newer.
* A hand-written client is the fallback if both fail the spike. It would duplicate
  what these crates already do, so it is the last resort.
* Whichever we choose: pin an exact version, and consider `cargo vet` or vendoring, because
  the crate sits on the path to secrets.

**Decision (D10): `keepassxc-client`.** Reasons: it handles reply-nonce checking,
incremental JSON framing and signal interleaving, which `proxy-lib` does not. It is
also the crate that already has tests for the NaCl and association round trips. Risk:
young, single maintainer, so it is pinned exactly and any upgrade is deliberate.

Verified on this machine on 2026-11-20 with a throwaway crate outside the repo:

* Socket discovery finds `$TMPDIR/org.keepassxc.KeePassXC.BrowserServer`.
* `change-public-keys` succeeds (key exchange, no approval required).
* `get-databasehash` returns **"database is locked"**. The user's database was locked at
  the time, so no entry was touched. Live tests need the database unlocked.
* `kpxc-client` builds with the pinned dependency under our mise-managed Rust.

### Option C: reference scheme (what `fnox.toml` `value` means)

`get-logins` matches on the entry's **URL field**, not on its title or group path.

| Option                                                                               | Example                                                | Notes                                                                                                                      |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| `fnox://<name>` in the entry's URL field, or in a Browser Integration additional URL | `value = "fnox://github-token"`                        | **Live-verified** on KeePassXC 2.8.0 for primary and additional URLs (see results below). Use lowercase, hyphenated names. |
| `keepassxc://by-path/<group>/<title>` (native selector found in source)              | `value = "keepassxc://by-path/fnox-test/github-token"` | Live-verified for entries that have a URL. A matched URL-less entry crashed KeePassXC during one test. See live results.   |
| A syntactically normal URL                                                           | `https://fnox.invalid/github-token`                    | Likely matches reliably but is uglier                                                                                      |
| Title or group path (as fnox's `keepass` provider does)                              | `work/github`                                          | Not available through `get-logins`; would need a workaround                                                                |

Also consider returning a non-password field (`login`, or a `KPH:` custom field) by
adding a field selector to the reference, for example `fnox://name#login`.

#### Source reading: KeePassXC 2.8.0 (installed version, read 2026-11-20)

Read from the `2.8.0` tag of `src/browser/BrowserService.cpp`, `BrowserAction.cpp`,
`gui/UrlTools.cpp` and `core/Entry.cpp`. This is code reading, not a live test.

* **No scheme gate.** `get-logins` only rejects an empty `url`. Any non-empty value, including
  `fnox://…`, goes straight to the matcher.
* **Additional URLs are checked.** `shouldIncludeEntry` tests the primary URL first, then each
  additional URL. Additional URLs are stored as `KP2A_URL_<n>` attributes.
* **Host-based matching.** For a `fnox://name` URL, the host is `name`. A single-label host has
  no TLD, so its base domain is the host itself. The base-domain test is therefore an exact
  comparison, and the following `endsWith` test is only reached after that.
* **Hostnames are lowercased by `QUrl`.** Use lowercase names.
* **Dotted names widen the match.** If nothing matches, `removeFirstDomain` strips the first
  label and retries, so `fnox://a.b.c` can match entries for `b.c`. Use hyphens, not dots.
* **Access control is per entry, keyed by host.** The first lookup for an unapproved entry shows
  a confirmation dialog unless "remember" is ticked. For `fnox://name` the stored host is `name`.
* **`keepassxc://by-path/<group>/<title>`** matches `entry->path()` (group hierarchy without
  root, plus title). The access host for these lookups is `by-path`, so the approval dialog
  will show that value rather than the entry name.

### Option D: association storage

After the one-time approval in KeePassXC, the client must remember the association id and
identification key.

**Important and verified from source and spec:** only the *public* half of the
identification key is ever sent. The spec itself notes this as "a tiny flaw". The
association id plus public key therefore behaves as a **bearer credential**: any local
process that has it can ask KeePassXC for entries. `keepassxc-client`'s README claims the
idKey secret "proves we are the associated client", which is not how the protocol works. The
`proxy-lib` README states it accurately.

KeePassXC's per-entry confirmation dialog is the real consent boundary. **Unverified:**
exactly when it prompts for a client that is already associated. Test this in the spike.

| Option                              | Pros                                                    | Cons                                                     |
| ----------------------------------- | ------------------------------------------------------- | -------------------------------------------------------- |
| macOS Keychain                      | Not readable by other processes without a prompt or ACL | More code; needs the Security framework or `security(1)` |
| File under `~/.config/` with `0600` | Simple and inspectable                                  | Readable by anything running as the user                 |

**Decided (2026-11-20, revised the same day): a `0600` file under `$XDG_DATA_HOME`.** It is
`$XDG_DATA_HOME/fnox-plugin-keepassxc/association.json` (falling back to
`$HOME/.local/share`), with the directory at `0700`. It holds the association id and the 32-byte
identification key, and it is a bearer credential. It is kept out of the dotfiles repo, and it
works the same on macOS and Linux with no prompts. The Keychain stays an option behind the same
interface. The location is data, not state, because the association is created by the user's
approval in KeePassXC, and losing it would surprise the user. Still open: whether one association
is shared across projects or whether each project gets its own.

#### Security tradeoff and precautions

The file holds the association ID and the 32-byte identification key, so it is a bearer credential
(see the source note above). The secret half is never sent on the wire, but the client needs it to
rebuild the public key, so it has to be stored.

Tradeoff, compared with a Keychain item:

* Any process running as the same user can read the file and then ask a running, unlocked
  KeePassXC for `fnox://` entries. The Keychain would add OS-level access control. The file has
  none.
* The exposure is limited to entries with a `fnox://` URL, and KeePassXC's per-entry approval still
  applies. The master password is never in the file.
* Same-user malware, root, and backups that copy the home directory can all read the file. The
  file permissions do not protect against those.

Precautions, as documented in the README:

* Keep KeePassXC locked when not in use.
* Leave **Remember** unticked in the access dialog, so each lookup needs approval. **Unverified:**
  the plan's source reading (2.8.0) says ticking it saves the approval. That would make later
  lookups silent. Check the dialog wording and behaviour during the live run, and update this
  section and the README.
* Tag only the entries that fnox needs.
* Keep the file out of version control, sync folders, and shared backups.
* To revoke, delete the file and remove the association from KeePassXC's Browser Integration
  settings. **Unverified:** the location of that setting in the installed KeePassXC version.

File handling, as implemented in `client/src/store.rs`: the app's directory is set to `0700`, and
the file is `0600`, reset on every save. Parent directories such as `~/.local/share`, if they are
created, use the default mode.

#### Browser comparison (2026-11-20)

The KeePassXC browser extension holds the same kind of credential, so it is a useful comparison.
Sources:

* Protocol spec,
  [`keepassxc-protocol.md`](https://github.com/keepassxreboot/keepassxc-browser/blob/develop/keepassxc-protocol.md).
  It describes the permanent identification key, says it "should be stored safely by the browser",
  and notes that only the public half is used, which "might be a tiny flaw".
* Extension permissions, in the
  [`keepassxc-browser` README](https://github.com/keepassxreboot/keepassxc-browser). The `storage`
  permission is "for storing extension settings (always stored locally in the browser, they are
  never synced)".
* Issue #1884,
  [What goal does the protocol encryption serve?](https://github.com/keepassxreboot/keepassxc-browser/issues/1884).
  The protocol's author says the identification token is "merely an identification token", and that
  malware able to extract it from extension storage gets whatever keys are stored there. A
  maintainer says connection keys should move to the OS keychain rather than browser local storage,
  which has not happened yet.
* Issue #4802,
  [Document threat model of browser extension](https://github.com/keepassxreboot/keepassxc/issues/4802).
  A project member says no software on a compromised machine can guarantee security. A comment in
  the same thread says most clients, including many browser extensions, store these keys in
  plaintext. That comment's author is not confirmed.
* [`keepassxc-client` on crates.io](https://crates.io/crates/keepassxc-client), which advises
  storing the identification secret in an OS keyring, not on disk in the clear.

Findings:

* The browser stores its identification key in the extension's own storage. Our file is the
  equivalent. Neither is protected by more than the storage's own permissions. The browser storage
  format is not verified from source.
* Encryption on the native messaging channel protects against passive eavesdropping only. It does
  not stop same-user malware. Per-entry approval in KeePassXC is the real consent boundary.
* **Gap:** the extension sends the page URL in `get-logins`, and KeePassXC matches entries against
  it, so a different site cannot ask for an unrelated login. Our client sends a `fnox://` name, and
  any process holding the association can ask for any entry tagged that way. Tagging only the
  entries fnox needs limits this, but it does not remove it.

## Live test database (specification)

Dedicated KeePassXC database, never the real one. Use the Browser Integration settings and the
KeePassXC 2.8.0 defaults unless noted, and record any change here.

Naming rules: lowercase, hyphens only, no dots, and a `kpxc-test-` prefix on every reference.
Passwords must be unique random strings, never real credentials.

| Entry (group `fnox-test`)  | Primary URL                | Browser Integration additional URL | Username          | Password                 | Purpose                                                  |
| -------------------------- | -------------------------- | ---------------------------------- | ----------------- | ------------------------ | -------------------------------------------------------- |
| `fnox-primary`             | `fnox://kpxc-test-primary` | none                               | `test-primary`    | `primary-pw-<random>`    | Case 1: reference in the primary URL                     |
| `fnox-additional`          | *(empty)*                  | `fnox://kpxc-test-additional`      | `test-additional` | `additional-pw-<random>` | Case 2: reference only in an additional URL              |
| `fnox-dup-1`               | `fnox://kpxc-test-dup`     | none                               | `test-dup-1`      | `dup-1-pw-<random>`      | Two matches for one reference                            |
| `fnox-dup-2`               | `fnox://kpxc-test-dup`     | none                               | `test-dup-2`      | `dup-2-pw-<random>`      | Same                                                     |
| `fnox-decoy-suffix`        | `fnox://test-primary`      | none                               | `test-decoy`      | `decoy-pw-<random>`      | Must NOT match `kpxc-test-primary` (suffix trap)         |
| `fnox-bypath` *(optional)* | *(empty)*                  | none                               | `test-bypath`     | `bypath-pw-<random>`     | Alternative: `keepassxc://by-path/fnox-test/fnox-bypath` |

Expected results, one row per lookup:

| Lookup URL                    | Expected                                                             |
| ----------------------------- | -------------------------------------------------------------------- |
| `fnox://kpxc-test-primary`    | Exactly `fnox-primary`                                               |
| `fnox://kpxc-test-additional` | Exactly `fnox-additional`                                            |
| `fnox://kpxc-test-dup`        | Both `fnox-dup-1` and `fnox-dup-2` (client should treat as an error) |
| `fnox://kpxc-test-missing`    | None (`errorCode 15`, KC returns an empty vec)                       |
| `fnox://kpxc-test-prim`       | None (no prefix matching)                                            |
| Database locked               | `DatabaseLocked` (errorCode 1)                                       |

Test-run prerequisites:

* Browser Integration is enabled in KeePassXC, and the test database is the active, connected
  database. Keep "search in all databases" off, or confirm no other open database uses these names.
* Each first lookup per entry shows an access dialog. Tick **Remember** for the test entries, or
  approve each one.
* Record the values of "Match URL scheme" and "Return only best-matching credentials" with each
  run. Both change the results.

### Live results (2026-11-20)

Association name: `fnox-test-tw`. Associated in-process only, with nothing written to disk.
Each lookup returned the `uuid` and `login` fields. Passwords were not printed.

| Lookup URL                                  | Result                                | Matches expected?                                                      |
| ------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------- |
| `fnox://kpxc-test-primary`                  | `fnox-primary` (group `fnox-test`)    | Yes                                                                    |
| `fnox://kpxc-test-additional`               | `fnox-additional` (group `fnox-test`) | Yes. Confirms the additional URL path                                  |
| `fnox://kpxc-test-dup`                      | `fnox-dup-1` and `fnox-dup-2`         | Yes. The decoy `fnox://test-primary` was not returned                  |
| `fnox://kpxc-test-missing`                  | 0 entries                             | Yes                                                                    |
| `fnox://kpxc-test-prim`                     | 0 entries                             | Yes. No prefix matching                                                |
| `keepassxc://by-path/fnox-test/fnox-bypath` | 0 entries                             | **Unresolved.** Retested after the user created `fnox-bypath`, still 0 |

Retest, 2026-11-20 (second association `fnox-text-tw`): the `fnox://kpxc-test-primary` control still
returned `fnox-primary`, so the connection works. The `by-path` lookup still returned 0.

KeePassXC 2.8.0 builds the `by-path` key from `Entry::path()`. That is the group hierarchy with the
root group removed, plus the entry title. The URL must therefore name the exact group and title. An
entry at the root of the database would need `keepassxc://by-path/fnox-bypath`. Open questions:
which group the entry sits in, and whether an access prompt appeared and was approved.

Third run (association `fnox-test-tw`, again approved): the user confirmed group `fnox-test`, title
`fnox-bypath`, and an approved access prompt. Results:

| Lookup URL                                   | Result                   |
| -------------------------------------------- | ------------------------ |
| `keepassxc://by-path/fnox-test/fnox-bypath`  | 0 entries                |
| `keepassxc://by-path/fnox-test/fnox-primary` | `fnox-primary` (1 entry) |
| `keepassxc://by-path/fnox-bypath`            | 0 entries                |

**Root cause: the entry title has a leading space.** The title is `" fnox-bypath"`. KeePassXC builds
the path from the title as stored, so the entry's path is `fnox-test/ fnox-bypath`. The lookup for
`fnox-test/fnox-bypath` could never match.

Fourth run (association `fnox-test-tw`), after the user added `fnox://fnox-bypath` as the URL:

| Lookup URL                                                    | Result                                              |
| ------------------------------------------------------------- | --------------------------------------------------- |
| `keepassxc://by-path/fnox-test/fnox-bypath`                   | 0 entries (still, because of the space)             |
| `fnox://fnox-bypath`                                          | `" fnox-bypath"` (1 entry). The URL works           |
| `keepassxc://by-path/fnox-test/fnox-primary`                  | `fnox-primary` (1 entry)                            |
| `keepassxc://by-path/fnox-test/ fnox-bypath` (with the space) | `" fnox-bypath"` (1 entry). Confirms the root cause |

Checks on the entry: the username equals the title, including the leading space. The password starts
with the title text, without the space.

**Correction:** the leading space explained the earlier `by-path` misses. The later crash (below)
shows that a URL-less entry does break KeePassXC once it is matched by path. The earlier conclusion
that the URL made no difference was true only for the space-affected runs.

Fifth run (association `fnox-test-tw`), after the user removed the title and username spaces and
removed the URL from `fnox-bypath`:

| Lookup URL                                   | Result                                                   |
| -------------------------------------------- | -------------------------------------------------------- |
| `keepassxc://by-path/fnox-test/fnox-bypath`  | **KeePassXC crashed**. Connection closed, KeePassXC gone |
| `fnox://fnox-bypath`                         | Not run. KeePassXC had already crashed                   |
| `keepassxc://by-path/fnox-test/fnox-primary` | Not run. Broken pipe                                     |

Crash report: `~/Library/Logs/DiagnosticReports/KeePassXC-2026-11-20-161358.ips`. It is a segfault
(`EXC_BAD_ACCESS`, null-address read) on the main thread, during a socket read. The binary has no
symbols in the report, so the faulting function is **not confirmed**. The likely candidate is the
unguarded `*std::max_element` over an empty URL list in `sortPriority`, which is reached only when a
URL-less entry matches. A null read there would fit the report.

Consequences:

* Do not use `keepassxc://by-path/` against URL-less entries, and do not expect KeePassXC to survive
  the lookup.
* Consider reporting this to KeePassXC, after reproducing it with a minimal test and confirming the
  symbol.
* Whether the crash risks the database has not been checked. Changes made in the GUI may not have
  been saved.

Still to do: relaunch KeePassXC, unlock the test database, check that the title and username spaces
were saved, and decide whether `fnox-bypath` should have a URL again.

Sixth run, after the user relaunched KeePassXC and reopened the test database (association
`fnox-test-tw`, approved again). KeePassXC stayed up. Only `fnox://` lookups were run, so the
URL-less entry was never matched by path:

| Lookup URL                    | Result                                  |
| ----------------------------- | --------------------------------------- |
| `fnox://kpxc-test-primary`    | `fnox-primary` (1)                      |
| `fnox://kpxc-test-additional` | `fnox-additional` (1)                   |
| `fnox://kpxc-test-dup`        | `fnox-dup-1`, `fnox-dup-2` (2)          |
| `fnox://kpxc-test-missing`    | 0                                       |
| `fnox://kpxc-test-prim`       | 0                                       |
| `fnox://fnox-bypath`          | 0. Expected, since the entry has no URL |

Not yet checked: whether the title and username spaces were saved. The protocol can't show this for
a URL-less entry without risking another crash, so the user should confirm this in the KeePassXC UI.

Evidence on URLs and `by-path`: in the fourth run, `fnox-bypath` had the URL `fnox://fnox-bypath`
and resolved by path when the path was right (`fnox-test/ fnox-bypath`). The crash happened only
when the URL was absent. So a URL on the entry is the safe configuration for path lookups.

Seventh run, with the test database locked by the user. KeePassXC stayed up.

| Request                        | Result                                 |
| ------------------------------ | -------------------------------------- |
| key exchange                   | OK. Works without an unlocked database |
| `get-databasehash`             | `DatabaseLocked`                       |
| `associate`                    | `DatabaseLocked`                       |
| `test-associate` (unknown key) | `DatabaseLocked`                       |

Not run: `get-logins` while locked. KC needs an association before it sends that request, and a
fresh association needs an unlocked database. Locked `get-logins` behaviour is therefore still
unverified. Since KC maps errorCode 1 to `DatabaseLocked` for every request, it is likely to behave
the same way, but this needs checking. The check requires an association made while unlocked, so the
association from before the lock would need to be saved.

Eighth run, after the user unlocked the database and re-added `fnox://fnox-bypath` to `fnox-bypath`.
The association request returned `Action cancelled or denied` (error code 6), so no lookups ran.
KeePassXC stayed up. Likely cause: the approval prompt was dismissed or not seen. Retry needed.

Ninth run, after the user re-approved the association (`fnox-test-tw`). KeePassXC stayed up.

| Lookup URL                                   | Result                                           |
| -------------------------------------------- | ------------------------------------------------ |
| `keepassxc://by-path/fnox-test/fnox-bypath`  | `fnox-bypath` (1). Resolves with the URL present |
| `fnox://fnox-bypath`                         | `fnox-bypath` (1). Same UUID as the path lookup  |
| `keepassxc://by-path/fnox-test/fnox-primary` | `fnox-primary` (1)                               |

`fnox-bypath`: the title and username no longer have a leading space. The username equals the title,
and the password starts with the title. The password length is 22.

Open item: the crash on a URL-less `by-path` match is unexplained at the code level (the crash
report has no symbols). Reproducing it would need a second URL-less test entry, which I have not
created. Not needed for the test plan, but worth reporting upstream if the behaviour is confirmed.

## Known risks and open questions

1. **Custom-scheme URL matching** (Option C) is verified live on KeePassXC 2.8.0: `fnox://`
   lookups matched primary and additional URLs. The `keepassxc://by-path/` selector also worked
   for entries with a URL. Keep lowercase, hyphenated names; dots can widen host matching.
2. **macOS transport** is verified live with `keepassxc-client` on this machine: socket discovery,
   association, and lookups worked. `keepassxc-proxy-lib` was not used and is no longer a candidate.
3. **Confirmation dialog** for an already-associated client is live-verified on the test entry:
   uncached lookup with `FNOX_DAEMON=off` prompted. Whether **Remember** preserves approval across
   later requests has not been isolated in a test.
4. **Prompting.** `get-logins` can pop a dialog in KeePassXC; a non-interactive `fnox hook-env` or
   `mise` activation must not hang. Decide on a timeout and clean failure.
   **Live result, test entry only:** with `FNOX_DAEMON=off`, an uncached lookup prompted in
   KeePassXC; with a warm daemon cache, lookup returned without a prompt. fnox can avoid repeat
   provider lookups with its optional memory-only
   [daemon cache](https://fnox.jdx.dev/guide/daemon.html), but it is disabled by default; the first
   read of each uncached value may still prompt. Cache hits last until expiry or daemon shutdown.
   Any process running as the same user can request cached values. The independent persistence
   effect of KeePassXC's **Remember** checkbox remains unverified.
5. **Multiple databases / multiple matches.** Multiple matches map to an ambiguity error in the
   client and are covered by in-process tests. Confirm KeePassXC's "search in all databases"
   setting and the intended database behavior for daily use.
6. **Test double resolved:** lookup tests use `keepassxc-client`'s `MockTransport` with an
   in-process fake KeePassXC responder. The Lua/Rust integration test covers the process boundary
   without a KeePassXC socket.
7. **Supply chain.** Young, single-maintainer crates on a secrets path. Pin and audit.
8. **Secret exposure in our own process.** The client prints the secret on stdout and fnox
   puts it in the environment. Avoid logging it; consider zeroizing buffers. Review before
   release.
9. **fnox/mise wiring.** `fnox provider test`, test-entry `fnox get` with fresh association, and
   task-scoped mise injection are verified using the local plugin. The mise task only asserts that
   the variable is nonempty; use by a real application and `fnox exec` remain untested. mise's
   experimental task-scoped `secrets.fnox` integration does not inject secrets into the shell or
   `mise env`. It requires 2026.10.4 or newer; installed mise is 2026.10.17. fnox's direct mise
   cache reads also require fnox 1.50.0 or newer. See the
   [fnox mise guide](https://fnox.jdx.dev/guide/mise-integration.html) and
   [mise fnox docs](https://mise.jdx.dev/environments/secrets/fnox.html). An earlier get used
   relative `XDG_DATA_HOME=./data`; the client ignored it and used the default path. The successful
   fresh-association run used an absolute path under the repository's git-ignored `tmp/` directory;
   use an absolute temporary path outside the repo next time.

    The direct mise daemon connection is mismatched: installed mise 2026.10.17 bundles
    `fnox-client` 1.40.0 (wire protocol 6), while fnox 1.50.0 uses protocol 8. Protocol version is
    part of the socket name, so these clients seek different sockets. mise 2026.11.12 bundles
    `fnox-client` 1.50.0, matching fnox. No `[daemons]` config is needed; a socket directory
    override alone cannot fix this protocol mismatch. Downgrading fnox to 1.40.0 would match the
    wire version, but mise docs require fnox 1.50.0 or newer for direct daemon reads. Decision D13
    accepts the CLI fallback: `FNOX_DAEMON=on` uses fnox's own cache and avoids repeat prompts.
    Direct socket optimization is not required for current use.

## Next steps

The original prototype steps are complete. Remaining work to use fnox for tokens:

1. Test a real application consuming a secret through a mise task.
2. Decide whether standalone commands also need `fnox exec`.
3. Choose production cache TTL; test config's `10m`/`30m` limits are temporary. Initial uncached
   lookups still resolve through the provider and may prompt.
4. Tag each desired KeePassXC token entry with its `fnox://<name>` URL.
5. Link the preferred checkout/source with `fnox plugin link` or a git source in shared config.
6. Remove token values from `mise.toml` after the real fnox flow passes.
7. Add release tags and a published release only when release distribution is needed.

TODO items, not yet scheduled:

* TODO: Add configuration that opts out of storing the credential mapping at all. With it set,
  the client would not write the association, so there would be nothing on disk to read. Each
  use would then need a fresh association, with the KeePassXC approval each time. Decide the
  setting's name, its default, and where it is read from (environment variable or config file).
* TODO: Look at the Rust library that abstracts OS-specific key storage, such as the `keyring`
  crate, as a replacement for the file. It would cover the macOS Keychain, the Linux Secret
  Service, and Windows Credential Manager. Check its maintenance and supply-chain risk before
  adopting it, since it sits on the secrets path. Also check how each platform prompts for access.

## Re-evaluation (2026-11-20)

Question asked: with KC handling the parts Lua cannot, do we still need Rust, and does the
test structure still fit?

### Do we need Rust?

Yes, but the Rust we own shrinks to a thin CLI.

* fnox plugins can only run processes (`fnox.cmd`). They cannot open sockets or do the
  NaCl box, and KC does both. So something executable must sit between `provider.lua` and
  KeePassXC.
* KC is a library. Its only executable is an example that writes to the user's database,
  so it cannot serve as the binary.
* The binary we own is roughly:
  1. Parse the `fnox://<name>` reference (already implemented).
  2. Load the stored association (storage is still open, see Option D).
  3. Connect, call `get_logins(url)`, pick the one matching entry.
  4. Print the password on stdout, or write a diagnostic on stderr and exit with a code.
* We own **no protocol code**: no framing, crypto, nonce handling or signal handling.
* The Lua side is unchanged in role: it maps exit codes to fnox semantics (`nil`,
  `auth_error`, or `error`).

### Does the test structure still fit?

Partly. The layers are right, but the contents need to move.

| Layer                                | Keep or change       | Reason                                                                                                                                                                    |
| ------------------------------------ | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lua unit (busted, mocked `fnox.cmd`) | Keep                 | Cheap. Covers the mapping from exit codes to fnox behaviour                                                                                                               |
| Rust unit                            | Keep, extend         | Add `KxcError` to exit-code mapping. Pure and testable                                                                                                                    |
| Rust integration (lookup logic)      | **Add**              | Use KC's public `MockTransport` plus our own fake responder in-process. Covers found, not found, locked, not associated, multiple matches, and signals arriving mid-reply |
| Lua-to-Rust integration              | Narrow               | Only test process-boundary behaviour that needs no KeePassXC (for example, bad reference or no socket). The echo-stub assertion must go                                   |
| Live check against real KeePassXC    | **Add, manual only** | Read-only. Not run in CI                                                                                                                                                  |

Points to note:

* **KC's fake KeePassXC is private.** `FakeKxc` sits in a `#[cfg(test)]` module and cannot
  be imported. Our responder will duplicate some of its crypto (about 100 lines). That is the
  one real duplication, and it is confined to test code.
* **Do not reuse KC's `kxc_smoke` example as-is.** It writes and deletes entries in the
  user's database, and stores the id key secret in plaintext. Our live check should be
  read-only.
* **Association storage is still open.** KC's README recommends the OS keyring. We would
  need to choose between the keychain and a `0600` file, and decide whether one association
  is shared across projects.

### Open items this re-evaluation leaves

1. `fnox://` URL matching is still **unverified**. It needs an unlocked database and a
   tagged test entry, plus one approval click in KeePassXC to associate.
2. Association storage (Option D).
3. Whether to keep the Lua integration test at all, or cover the process boundary only in
   Rust.

## Log

### 2026-11-19

* Surveyed fnox: the built-in `keepass` provider opens the `.kdbx` directly (see above). The
  Lua plugin provider is the only extension point.
* Assessed the KeePassXC Browser protocol against `keepassxc-cli`; chose the protocol first
  (D1) and recorded the CLI as a fallback to raise with the user before use.
* Scaffolded the repo: `provider.lua`, `spec/`, `client/`, `mise.toml`, test scripts (D2 to D8).
* Added the Lua-to-Rust integration test. Committed the scaffold; the integration-test
  commit is pending (see the 2026-11-20 entry).
* Installed `lua@5.4` and `rust@stable` via mise; busted in a project-local `lua_modules/`.

### 2026-11-20

* Read both candidate crates in full. Findings are in Option B.
* **Correction:** an earlier statement that `keepassxc-proxy-lib` is "used by
  `keepassxc-client`" was wrong. They are independent implementations.
* **Correction:** an earlier `which keepassxc` returned "not found" because it searched `PATH`
  only. KeePassXC is installed at `/Applications/KeePassXC.app` and its browser socket exists.
* Noted that the identification key's secret half is never used on the wire (Option D).
* Flagged unverified assumptions: custom-scheme URL matching, confirmation-dialog behaviour,
  macOS behaviour of both crates.
* The commit of the integration-test files was requested but declined twice at the `git commit`
  permission prompt (`git commit*` is `ask` in `~/.config/opencode/opencode.jsonc`). The files
  remain staged.
* Decided D10 (KC as the base, pinned `=0.1.0`) and D11 (thin Rust CLI). Added the
  re-evaluation section above.
* Verified KC's key exchange against the real socket on this machine; the database was
  locked, so `get-databasehash` failed as expected. No entries were read or written.
* Added `keepassxc-client = "=0.1.0"` to `client/Cargo.toml`; it builds. Cargo emits an
  unused-dependency warning until the CLI uses it.
* Not committed. `git commit` is still gated by the permission prompt.
* Read the KeePassXC 2.8.0 matching code (installed version). Found that `get-logins` has no
  scheme gate, that additional URLs are checked, and that `keepassxc://by-path/` is a native
  selector. Recorded under Option C.
* Wrote the live test database specification. Awaiting the user's test database.
* Added `AGENTS.md` (commit `7ac43db`). It sets the red/green rules, the `#RED` and `#GREEN`
  tags, the `rumdl` check before Markdown commits, and the no-push rule.
* Plan step 2 is done. The client maps `DatabaseLocked` and `NotAssociated` to exit 4, the
  unreachable errors (`SocketNotFound`, `ConnectionClosed`, `Io`) to exit 5, and the rest to exit
  1. Exit 2 remains the usage error in `main.rs`. Commits `66b3942`, `5951e8f`, `f63047b`, and
  `30bce0b`. The `Json` and `Base64` variants are untested, because building them needs direct
  dependencies.
* Plan step 3 is done. `lookup` asks KeePassXC for `fnox://<name>` and returns `Found(password)`,
  `NotFound`, or `Ambiguous(count)`. Locked and not-associated states come back as errors. The
  tests in `client/tests/lookup.rs` run in-process against a fake KeePassXC built on the library's
  public crypto API. They cover six cases, including a signal that arrives mid-reply. Commits
  `54d9bb5` and `b7c181d`. The dev-dependencies `serde_json` and `tokio` are pinned to the versions
  already in the lock file.
* Plan step 1 is done. `provider.lua` branches on the client's exit status instead of matching
  stderr. Status 4 calls `fnox.auth_error`, and status 5 raises "KeePassXC is not reachable". Status
  1 and 2 raise. Other statuses return nil, including not-found, because exit 3 is not defined yet.
  Commits `c13354c` (red) and `af37425` (green).
* Ran `rumdl fmt` on this file so that `rumdl check` passes. The changes are layout only: list
  markers, table padding, and wrapping.
* Plan step 4 is done in the code, but not yet verified live. The association is stored in
  `$XDG_STATE_HOME/fnox-plugin-keepassxc/association.json` with mode `0600` (Option D). The
  client maps lookup outcomes to exit codes, with `NotFound` to 3. The CLI supports `--version`,
  `associate`, and `fnox://<name>`, and every failure path maps to an exit code. Commits: storage
  `7e9b703` (red) and `5b1ce83` (green); exit 3 `0104d52` (red) and `2c2a8cc` (green); CLI
  `b4af031` (red) and `f897b46` (green).
* Not verified: the success path (`Found`, password on stdout) against a real KeePassXC. The
  automated tests use a fake KeePassXC, and the live `associate` needs a click in KeePassXC.
* Known gaps: `5b1ce83` includes a one-line `rustfmt` change in new code, which `AGENTS.md`
  would normally keep in its own commit. `scripts/test-integration.sh` leaves two temporary
  directories behind on each run, because `exec` replaces the shell.

### 2026-11-20 (later)

* Moved the association file from `$XDG_STATE_HOME` to `$XDG_DATA_HOME`, with the fallback at
  `$HOME/.local/share`. The association is created by the user's approval in KeePassXC, so losing
  it would surprise the user. That makes it user data, not state, by the XDG spec's distinction.
  The earlier entries above that name `$XDG_STATE_HOME` describe the state. They are not
  edited. Commits: `58f10e2` (red) and `b39faf5` (green).
* Corrected two statements that were out of date. The CLI is wired (`f897b46`), so the README and
  the "Current code state" paragraph no longer say the binary only echoes its input. The live check
  against KeePassXC is still pending.
* Recorded how the browser extension stores its identification key, with links to the protocol
  spec and to KeePassXC issues #1884 and #4802 (Option D). The gap it exposes is that our client
  has no origin binding, so any process with the association can ask for any `fnox://` entry.
* Documented the association file's security tradeoff, the precautions, and its location. The
  README has a Security section, and Option D has the full detail. The **Remember** behaviour is
  taken from the 2.8.0 source reading and has not been checked live, so it is marked unverified.
* Added a readiness checklist near the top of the plan, for getting to live use with tokens read
  through fnox. Recorded two findings: with Remember unticked, each lookup prompts again, which
  matters for `mise` activation (Known risks, item 4), and fnox is installed through mise, but
  `kpxc-client` is not on PATH (Known risks, item 9).
* Decided D12's intended lookup order: `client_bin`, then
  `<plugin_dir>/client/target/release/kpxc-client`, then PATH. Added the mise build task and
  documented development (`fnox plugin link`) and git-source wiring. Review on 2026-11-23 found
  that only `client_bin` and PATH are implemented; plugin-relative lookup remains open. Whether
  fnox Lua can check for a file is unverified. `auth_command` needs its own path because fnox runs
  it outside the plugin.
* Live steps, recorded as complete by the user on 2026-11-23: `kpxc-client associate`, approved in
  KeePassXC, and `kpxc-client fnox://<name>` for a test entry and for a real token entry. The
  output was not captured in this session, so no lookup values are recorded here. The
  readiness checklist ticks these two steps only. Tagging each token entry is still open.
* Added the `build` mise task. It builds the release binary into `client/target/release`, the
  path D12 searches. That directory is already git-ignored. The existing `test:lua` and
  `test:rust` tasks pass through mise, and `test` runs both plus the integration suite.

### 2026-11-23

* The user reports completing live association and lookup with the local CLI: one test entry and
  one real token entry. Exact command output and values were not captured, and no secret is
  recorded here. The readiness checklist marks the reported steps done; tagging every token and
  testing through real fnox remain open.
* Reviewed docs against the code. Corrected the binary lookup description: `provider.lua` uses
  `options.client_bin`, otherwise bare `kpxc-client` on PATH. It does not search `ctx.plugin_dir`
  or `client/target/release` yet. The mise build task creates a local release binary, but fnox does
  not discover it automatically.
* Refreshed the known risks and next steps. `fnox://` matching, macOS transport, lookup tests, and
  process-boundary tests are verified; real fnox configuration, binary discovery, prompt policy,
  and environment handoff remain open.
* Implemented D12's plugin-relative fallback. `provider.lua` now tries `options.client_bin`, the
  release binary under `ctx.plugin_dir`, then PATH. Verified with fnox 1.50.0: `fnox provider test`
  loaded the repo as a local plugin and ran the release binary without a `client_bin` option or
  PATH install. Commits: `21bd9c6` (red) and `7523892` (green). `fnox get` remains untested.
* Later, the user reported `fnox get KPXC_TEST_PRIMARY` returned the expected test-entry value.
  The command set `XDG_DATA_HOME=./data`, which is relative; the client ignored it and used
  `$HOME/.local/share`. No output value is recorded. Repeat with an absolute temporary data home
  to verify fresh association setup, and separately test environment handoff.
* Follow-up: the user reran with absolute `XDG_DATA_HOME="$PWD/data"` while using the test
  database. fnox ran `auth_command`, association succeeded, and the retried `fnox get` returned
  the test entry. The association file is under the git-ignored `tmp/data/` directory in this
  checkout; inspected permissions are `0700` for the app directory and `0600` for the file. No
  secret value is recorded. Environment handoff through `fnox exec` or mise remains untested.
* Read the official [fnox daemon docs](https://fnox.jdx.dev/guide/daemon.html),
  [fnox mise guide](https://fnox.jdx.dev/guide/mise-integration.html), and
  [mise fnox docs](https://mise.jdx.dev/environments/secrets/fnox.html). Documented behavior: the
  daemon is optional and disabled by default; its cache is memory-only. On a cache miss, mise calls
  fnox to resolve the value, which can run this provider and prompt. On a hit, supported mise
  versions can read from the daemon socket directly. The cache is accessible to same-user processes.
  Local versions: fnox 1.50.0, mise 2026.5.12; mise's task-scoped fnox secrets feature requires mise
  2026.10.4 or newer. This integration is not yet tested here.
* The user upgraded mise to 2026.10.17, above the minimum for its experimental fnox secrets
  integration. Next test: use a mise task scoped to `KPXC_TEST_PRIMARY`, with the fnox daemon
  disabled and `daemon_cache = false`, against the test database.

### 2026-11-24

* The user reports the mise task eventually succeeded with the test database. fnox first reported
  `DatabaseLocked`, then ran `auth_command`, associated successfully, retried, and the task's
  `test -n "$KPXC_TEST_PRIMARY"` assertion passed. The secret value is not recorded. The task also
  printed mise's hint that secret tasks do not receive stdin by default; this is informational for
  this non-interactive assertion. Set `interactive = true` only for tasks that need terminal input;
  mise then sends output directly to the terminal without redaction.
* The first direct task run appeared to wait on a process; the user ran `mise install` with the data
  directory set, then reran successfully. Root cause of the initial wait is unknown. The output
  proves task-level injection, but does not by itself prove repeated runs avoid caching/prompts.
* The user ran the mise task twice with `FNOX_DAEMON=on`; both debug logs said
  `fnox daemon not used (Absent); using the fnox CLI`, so neither log showed a direct mise daemon
  hit. A later `fnox daemon status --verbose` showed the daemon running with `KPXC_TEST_PRIMARY`
  cached.
* Clarification from the user: KeePassXC did not prompt with `FNOX_DAEMON=on`, but did prompt with
  it off. This strongly suggests the fnox CLI fallback used its daemon cache. Cache behavior works
  at the observed level; mise's direct daemon fast path had not yet been checked.
* Checked again with the daemon running and `KPXC_TEST_PRIMARY` cached. A fresh `MISE_DEBUG=1 mise
  run test-kpxc` succeeded and did not log `asking fnox` or invoke the CLI fallback; a subsequent
  daemon status still showed the key cached. This initially looked like a direct daemon read, but
  later dependency inspection found a wire-version mismatch in installed mise; direct socket use
  cannot be claimed until retested with a compatible mise.
* Socket mismatch found: mise 2026.10.17's lockfile pins `fnox-client` 1.40.0 (wire version 6),
  while fnox 1.50.0 uses wire version 8. The wire version is part of the socket-name hash. mise
  2026.11.12's lockfile pins `fnox-client` 1.50.0 (wire 8). Sources:
  [mise 2026.10.17 lockfile](https://github.com/jdx/mise/blob/v2026.10.17/Cargo.lock),
  [mise 2026.11.12 lockfile](https://github.com/jdx/mise/blob/v2026.11.12/Cargo.lock),
  [`fnox-client` 1.40.0](https://docs.rs/crate/fnox-client/1.40.0/source/src/lib.rs), and
  [`fnox-client` 1.50.0](https://docs.rs/crate/fnox-client/1.50.0/source/src/lib.rs). This likely
  explains why fnox sees its daemon while mise reports it missing. `[daemons]` in mise config is
  for service processes, not fnox's secret daemon. Upgrade Nix-managed mise to 2026.11.12+ and
  retest; a socket-directory override alone cannot reconcile different protocol hashes.
* Considered downgrading fnox to match mise's embedded fnox-client 1.40.0. mise's official fnox
  integration docs require fnox 1.50.0 or newer for direct daemon reads, so downgrade is not a
  documented route to the direct socket. It is unnecessary for cache behavior: fnox CLI fallback
  already uses the daemon cache and avoids repeated KeePassXC prompts.
* Decision D13: accept mise's fnox CLI fallback for cached task secrets rather than downgrade fnox
  or upgrade Nix-managed mise for direct socket reads. User observed no KeePassXC prompt with
  `FNOX_DAEMON=on` and a prompt with it off. The direct socket is an optional optimization, not a
  requirement for this project.
* Plan audit updated completed statuses: `fnox get`, fresh association via `auth_command`, and
  mise task-level injection are recorded as verified. The test confirms a value reaches the task,
  not a real consuming application. Added `fnox.toml` examples; a worked `mise.toml` task example,
  plugin-source choice, actual app use, token migration, and release remain open.
* Reviewed deployment options. fnox's documented git plugin installation clones the Lua plugin;
  it has no build/install hook, so it does not build this Rust client. fnox's built-in 1Password
  provider follows the external-CLI pattern and requires `op` on PATH. Lua plugins expose HTTP and
  process APIs, but not the local socket and NaCl crypto this KeePassXC Browser Integration client
  needs. No deployment option selected; D12 remains unchanged.
* Reviewed [`keepassxc-go` v1.7.0](https://github.com/MarkusFreitag/keepassxc-go/tree/v1.7.0): its
  MIT CLI has macOS/Linux release binaries and checksums, and source includes a Windows named-pipe
  transport. The CLI auto-associates on first use, stores its own key under the OS config
  directory, and reports all errors with exit 1. Static source review found its protocol client
  reads the socket once into a 4096-byte buffer and does not validate the response nonce increment.
  No switch selected; keep the current base unless a spike or upstream fixes address these gaps.
* Follow-up status review corrected the comparison and reference-scheme sections to reflect the
  live macOS KeePassXC tests. Updated fnox's daemon-cache description from official docs and
  recorded that the Remember checkbox's persistence effect remains unverified. D13 selects the
  working CLI fallback; direct socket access is an optional optimization.
