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

## Why not the built-in fnox `keepass` provider

**Documented** ([fnox keepass provider](https://fnox.jdx.dev/providers/keepass.html)):

* It opens the `.kdbx` file directly. KeePassXC is not involved at all.
* It needs the master password in `FNOX_KEEPASS_PASSWORD` / `KEEPASS_PASSWORD`, or a prompt,
  and decrypts the entire database in the fnox process.
* It cannot borrow an unlock performed in the KeePassXC GUI (Touch ID, YubiKey, and so on).

That is the discomfort this project exists to avoid.

## Decisions

| #   | Decision                                                                                                                                 | Date       | Notes                                                                                                              |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------ |
| D1  | Integrate with KeePassXC through the **Browser Integration protocol** first; do not move to other approaches without consulting the user | 2026-11-19 | User instruction. See [options](#option-a-keepassxc-access-mechanism)                                              |
| D2  | Implement as an **fnox Lua plugin** plus a **Rust client binary**                                                                        | 2026-11-19 | Lua cannot do sockets or NaCl crypto, so a helper binary is unavoidable                                            |
| D3  | Repo at `~/Projects/fnox-plugin-keepassxc`; fnox's own term is "provider", and shareable plugins are conventionally `fnox-plugin-<name>` | 2026-11-19 |                                                                                                                    |
| D4  | Layout: `provider.lua` and `spec/` at repo root, Rust crate in `client/`                                                                 | 2026-11-19 | `provider.lua` must be at the root for `owner/repo` git installs                                                   |
| D5  | Name the Rust piece **`client`** (not `bridge`, `wrapper`, `core`, `agent`)                                                              | 2026-11-19 | User choice; it is a protocol client                                                                               |
| D6  | No `lib/` directory until Lua helpers are actually needed                                                                                | 2026-11-19 |                                                                                                                    |
| D7  | Automated tests first, for both Lua and Rust                                                                                             | 2026-11-19 | `mise run test` runs unit, Rust, and integration suites                                                            |
| D8  | Rust for the client                                                                                                                      | 2026-11-19 | Single static binary, installable via mise                                                                         |
| D9  | Do not re-implement the protocol; use an existing crate                                                                                  | 2026-11-20 | Chose `keepassxc-client` (D10)                                                                                     |
| D10 | Use `keepassxc-client` (KC) as the protocol base, pinned exactly (`=0.1.0`)                                                              | 2026-11-20 | User decision. Supersedes the "behind a trait of our own" plan. See [Option B](#option-b-protocol-library)         |
| D11 | Keep a Rust binary, but make it a thin CLI over KC; it owns no protocol code                                                             | 2026-11-20 | Lua has no sockets or NaCl crypto, and fnox can only run processes. See [Re-evaluation](#re-evaluation-2026-11-20) |

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

### Current code state

The client has `parse_reference`, the exit-code mapping (`exit_code_for`), and `lookup`, which
resolves a name against KeePassXC through `keepassxc-client`. The `lookup` tests run against an
in-process fake. `main.rs` still echoes the parsed name and is not wired to `lookup` yet (plan
step 4). `provider.lua` maps the exit codes. No protocol code is written here; it comes from the
library.

### Tests that exist

| Suite                   | Command                                        | What it covers                             | Status after KC decision                                                    |
| ----------------------- | ---------------------------------------------- | ------------------------------------------ | --------------------------------------------------------------------------- |
| Lua unit (busted)       | `./scripts/test-lua.sh`                        | `provider.get` against a mocked `fnox.cmd` | Keep. Re-target at the exit-code contract                                   |
| Rust unit               | `cargo test --manifest-path client/Cargo.toml` | reference parsing                          | Keep. Add error-to-exit-code mapping                                        |
| Lua-to-Rust integration | `./scripts/test-integration.sh`                | real binary against the echo stub          | **Must change.** Asserts the stub's echo; will fail once real lookups exist |

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
* Every provider also accepts `daemon_cache`. **Unverified:** what it does and whether it
  helps here. Read the fnox docs before designing any caching.

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

Both crates were downloaded and read in full on 2026-11-20. Neither has been run against
KeePassXC.

|                                        | `keepassxc-proxy-lib` 0.2.0                                                                              | `keepassxc-client` 0.1.0                                     |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Repo                                   | git.dog/xx/keepassxc-proxy-lib                                                                           | gitlab.com/GaffaFix/keepassxc-client                         |
| Licence                                | MIT                                                                                                      | MIT OR Apache-2.0                                            |
| Model                                  | Blocking, std only (about 900 lines)                                                                     | Async on tokio (about 950 lines)                             |
| Socket read                            | Reads until a short read (<4096 bytes); can truncate a split reply or block on an exact multiple of 4096 | Incremental JSON stream parsing                              |
| Signals (`database-locked`/`unlocked`) | Not handled; surfaces as "No nonce in response"                                                          | Skipped and queued                                           |
| Reply nonce (+1) check                 | None                                                                                                     | Yes, with a tamper test                                      |
| "No logins found" (code 15)            | Error (`KeePassXCErrorCode`), mappable by us                                                             | Normalized to an empty vec                                   |
| Custom `KPH:` fields                   | `string_fields`                                                                                          | `string_fields`                                              |
| `triggerUnlock`                        | Yes (`test_associate(true)`)                                                                             | Not exposed                                                  |
| Association                            | `{name, public_key}` (serde)                                                                             | `{id, idKey}` and `IdKeyPair`                                |
| Own tests                              | Three trivial tests                                                                                      | Fake-KeePassXC round trips (private to the crate)            |
| macOS                                  | "untested"                                                                                               | Socket candidates include `$TMPDIR`; validated on Linux only |
| Maturity                               | 3 releases, about 360 downloads, one maintainer                                                          | 1 release, about 18 downloads, one maintainer                |
| Windows                                | Named pipe                                                                                               | Stub (irrelevant here)                                       |

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

| Option                                                                                                    | Example                                                | Notes                                                                                                                                     |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `fnox://<name>` in the entry's URL field, or in a Browser Integration additional URL (current assumption) | `value = "fnox://github-token"`                        | Explicit and visible in KeePassXC. **Source-supported, not live-verified** (see below)                                                    |
| `keepassxc://by-path/<group>/<title>` (new, found in source)                                              | `value = "keepassxc://by-path/fnox-test/github-token"` | No URL tagging needed. Uses KeePassXC's own hierarchy. Relies on a special case that the code comments describe as "the schema keepassxc" |
| A syntactically normal URL                                                                                | `https://fnox.invalid/github-token`                    | Likely matches reliably but is uglier                                                                                                     |
| Title or group path (as fnox's `keepass` provider does)                                                   | `work/github`                                          | Not available through `get-logins`; would need a workaround                                                                               |

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

Unresolved. Also decide whether one association is shared across projects or whether each
project gets its own.

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

1. **Custom-scheme URL matching** (Option C). Could force a different reference scheme.
2. **macOS behaviour of both crates.** The socket exists at
   `$TMPDIR/org.keepassxc.KeePassXC.BrowserServer` (mode `srwx------`, **verified** on this
   machine, KeePassXC at `/Applications/KeePassXC.app`), and both crates resolve that path
   in source, but neither has been run here.
3. **Confirmation-dialog behaviour** for already-associated clients (Option D).
4. **Prompting.** `get-logins` can pop a dialog in KeePassXC; a non-interactive `fnox hook-env`
   or `mise` activation must not hang. Decide on a timeout and clean failure.
5. **Multiple databases / multiple matches.** `get-logins` can return several entries. Define
   behaviour (error on ambiguity is the safe default).
6. **Test double.** The Lua-to-Rust integration test will need a fake KeePassXC on a real
   Unix socket. Neither library provides one (keepassxc-client's is a private in-process
   mock), so we will write the server half of the crypto once. Worth offering upstream.
7. **Supply chain.** Young, single-maintainer crates on a secrets path. Pin and audit.
8. **Secret exposure in our own process.** The client prints the secret on stdout and fnox
   puts it in the environment. Avoid logging it; consider zeroizing buffers. Review before
   release.
9. **mise integration.** Not yet designed: how fnox values reach mise
   (`mise` env integration versus `fnox exec`), and how mise installs the client binary.
   The user's first goal is environment variables for tokens; confirm the intended entry
   point.

## Next steps

Proposed, pending the user's go-ahead on each:

1. **Live spike** (needs the user): unlock KeePassXC, tag one test entry with a `fnox://`
   URL, approve the association once, then call `get-logins` with that URL and with an
   `https://` URL. Record the results and settle Option C and Option D.
2. Add the error-to-exit-code mapping and its unit tests.
3. Add lookup-logic tests in-process with `MockTransport` (see the table above).
4. Replace the echo stub with a real `get_logins` lookup and an `associate` subcommand.
   Wire `associate` to `auth_command` in the Lua provider.
5. Rewrite the Lua-to-Rust integration test for the real exit-code contract.
6. Add the read-only live check, run manually.
7. Distribution through mise, and a worked `fnox.toml` plus `mise.toml` example.

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
* Added `AGENTS.md` (commit `179313e`). It sets the red/green rules, the `#RED` and `#GREEN`
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
  Commits `1f4257c` (red) and `58a9df0` (green).
* Ran `rumdl fmt` on this file so that `rumdl check` passes. The changes are layout only: list
  markers, table padding, and wrapping.
