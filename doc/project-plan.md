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

- It opens the `.kdbx` file directly. KeePassXC is not involved at all.
- It needs the master password in `FNOX_KEEPASS_PASSWORD` / `KEEPASS_PASSWORD`, or a prompt,
  and decrypts the entire database in the fnox process.
- It cannot borrow an unlock performed in the KeePassXC GUI (Touch ID, YubiKey, and so on).

That is the discomfort this project exists to avoid.

## Decisions

| # | Decision | Date | Notes |
|---|----------|------|-------|
| D1 | Integrate with KeePassXC through the **Browser Integration protocol** first; do not move to other approaches without consulting the user | 2026-11-19 | User instruction. See [options](#option-a-keepassxc-access-mechanism) |
| D2 | Implement as an **fnox Lua plugin** plus a **Rust client binary** | 2026-11-19 | Lua cannot do sockets or NaCl crypto, so a helper binary is unavoidable |
| D3 | Repo at `~/Projects/fnox-plugin-keepassxc`; fnox's own term is "provider", and shareable plugins are conventionally `fnox-plugin-<name>` | 2026-11-19 | |
| D4 | Layout: `provider.lua` and `spec/` at repo root, Rust crate in `client/` | 2026-11-19 | `provider.lua` must be at the root for `owner/repo` git installs |
| D5 | Name the Rust piece **`client`** (not `bridge`, `wrapper`, `core`, `agent`) | 2026-11-19 | User choice; it is a protocol client |
| D6 | No `lib/` directory until Lua helpers are actually needed | 2026-11-19 | |
| D7 | Automated tests first, for both Lua and Rust | 2026-11-19 | `mise run test` runs unit, Rust, and integration suites |
| D8 | Rust for the client | 2026-11-19 | Single static binary, installable via mise |
| D9 | Do not re-implement the protocol; use an existing crate behind a trait of our own | 2026-11-20 | Which crate is [open](#option-b-protocol-library) |

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

- `provider.lua` is thin: run the client, map outcomes to fnox semantics. Return `nil` for
  "not found" (so fnox `default`/`if_missing` apply), `fnox.auth_error(...)` when the
  database is locked or not associated (fnox can then offer `auth_command`), and `error(...)`
  for everything else.
- `client/src/lib.rs` holds testable logic; `main.rs` is a thin CLI.
- Planned seam: a small trait of our own (for example `get(name) -> Result<Option<Secret>,
  LookupError>`) with the protocol library behind an adapter, so the library choice stays
  cheap to change and the Lua and CLI tests never see it.
- `get_batch` can later resolve several secrets in one process and one connection.

### Current code state

Only scaffolding exists. `parse_reference("fnox://name")` is implemented and tested; the
client's success path echoes the parsed name so the Lua-to-Rust process boundary can be
tested. **No protocol code has been written.**

### Tests that exist

| Suite | Command | What it covers |
|-------|---------|----------------|
| Lua unit (busted) | `./scripts/test-lua.sh` | `provider.get` against a mocked `fnox.cmd` |
| Rust unit | `cargo test --manifest-path client/Cargo.toml` | reference parsing |
| Lua-to-Rust integration | `./scripts/test-integration.sh` | real binary, real process spawn, via an `io.popen` shim |

## What fnox gives a plugin

**Documented** ([Lua plugins](https://fnox.jdx.dev/providers/lua.html)):

- Hooks: `get(ctx, ref)` (required), `get_batch`, `put`, `encrypt`, `test`.
- Modules: `fnox`, `fnox.http`, `fnox.json`, `fnox.cmd`, `fnox.log`, `fnox.base64`. There is
  no socket access and no NaCl crypto, hence the external client binary.
- `fnox.auth_error(msg)` plus a provider-level `auth_command` lets fnox offer to run a command
  (a natural home for `kpxc-client associate`).
- Plugin state does not persist between lookups (fnox may reload the plugin), so any caching
  must live in the client or fnox, not in Lua module variables.
- Every provider also accepts `daemon_cache`. **Unverified:** what it does and whether it
  helps here. Read the fnox docs before designing any caching.

## Options

### Option A: KeePassXC access mechanism

| | Browser Integration protocol (chosen first, D1) | `keepassxc-cli` |
|---|---|---|
| Talks to | The running, unlocked KeePassXC app over a local socket | The `.kdbx` file directly |
| Master password handled by us | No | Yes (arg, stdin, or env) |
| Reuses GUI unlock (Touch ID, YubiKey) | Yes | No |
| Exposure boundary | Only entries matched by `get-logins`, plus KeePassXC's confirmation dialog | Everything the credentials can decrypt |
| Works headless or in CI | No (needs a running, unlocked app) | Yes |
| Engineering cost | Higher: key exchange, NaCl box, association | Low: wrap the CLI |
| Same shape as fnox's built-in `keepass` provider | No | Effectively yes |

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

| | `keepassxc-proxy-lib` 0.2.0 | `keepassxc-client` 0.1.0 |
|---|---|---|
| Repo | git.dog/xx/keepassxc-proxy-lib | gitlab.com/GaffaFix/keepassxc-client |
| Licence | MIT | MIT OR Apache-2.0 |
| Model | Blocking, std only (about 900 lines) | Async on tokio (about 950 lines) |
| Socket read | Reads until a short read (<4096 bytes); can truncate a split reply or block on an exact multiple of 4096 | Incremental JSON stream parsing |
| Signals (`database-locked`/`unlocked`) | Not handled; surfaces as "No nonce in response" | Skipped and queued |
| Reply nonce (+1) check | None | Yes, with a tamper test |
| "No logins found" (code 15) | Error (`KeePassXCErrorCode`), mappable by us | Normalized to an empty vec |
| Custom `KPH:` fields | `string_fields` | `string_fields` |
| `triggerUnlock` | Yes (`test_associate(true)`) | Not exposed |
| Association | `{name, public_key}` (serde) | `{id, idKey}` and `IdKeyPair` |
| Own tests | Three trivial tests | Fake-KeePassXC round trips (private to the crate) |
| macOS | "untested" | Socket candidates include `$TMPDIR`; validated on Linux only |
| Maturity | 3 releases, about 360 downloads, one maintainer | 1 release, about 18 downloads, one maintainer |
| Windows | Named pipe | Stub (irrelevant here) |

Trade-off summary:

- `proxy-lib` is the better shape for a one-shot CLI but its receive path is not reliable in
  edge cases, and those are private functions we cannot patch from outside. We would need an
  upstream PR, a fork, or vendoring.
- `keepassxc-client` handles those edge cases but brings tokio and is newer.
- A hand-written client is the fallback if both fail the spike. It would duplicate
  what these crates already do, so it is the last resort.
- Whichever we choose: pin an exact version, and consider `cargo vet` or vendoring, because
  the crate sits on the path to secrets.

**Decision pending**: see [Next steps](#next-steps).

### Option C: reference scheme (what `fnox.toml` `value` means)

`get-logins` matches on the entry's **URL field**, not on its title or group path.

| Option | Example | Notes |
|--------|---------|-------|
| `fnox://<name>` in the entry's URL field (current assumption) | `value = "fnox://github-token"` | Explicit and visible in KeePassXC. **Unverified** that KeePassXC will match a non-http(s) scheme |
| A syntactically normal URL | `https://fnox.invalid/github-token` | Likely matches reliably but is uglier |
| Title or group path (as fnox's `keepass` provider does) | `work/github` | Not available through `get-logins`; would need a workaround |

Also consider returning a non-password field (`login`, or a `KPH:` custom field) by
adding a field selector to the reference, for example `fnox://name#login`.

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

| Option | Pros | Cons |
|--------|------|------|
| macOS Keychain | Not readable by other processes without a prompt or ACL | More code; needs the Security framework or `security(1)` |
| File under `~/.config/` with `0600` | Simple and inspectable | Readable by anything running as the user |

Unresolved. Also decide whether one association is shared across projects or whether each
project gets its own.

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

1. **Spike** (throwaway crate outside this repo): try each library against the real KeePassXC.
   Connect, associate once (one approval click), call `get-logins` with an `https://` URL and
   a `fnox://` URL, and observe the confirmation dialog. Record results in the log and settle
   Options B, C, and D.
2. Add the trait seam and adapter to `client/`, driven by tests with a fake server.
3. Implement `associate` as a client subcommand and wire it to `auth_command`.
4. Replace the echo stub with a real lookup; update the Lua exit-code mapping and tests.
5. Distribution through mise, and a worked `fnox.toml` plus `mise.toml` example.

## Log

### 2026-11-19

- Surveyed fnox: the built-in `keepass` provider opens the `.kdbx` directly (see above). The
  Lua plugin provider is the only extension point.
- Assessed the KeePassXC Browser protocol against `keepassxc-cli`; chose the protocol first
  (D1) and recorded the CLI as a fallback to raise with the user before use.
- Scaffolded the repo: `provider.lua`, `spec/`, `client/`, `mise.toml`, test scripts (D2 to D8).
- Added the Lua-to-Rust integration test. Committed the scaffold; the integration-test
  commit is pending (see the 2026-11-20 entry).
- Installed `lua@5.4` and `rust@stable` via mise; busted in a project-local `lua_modules/`.

### 2026-11-20

- Read both candidate crates in full. Findings are in Option B.
- **Correction:** an earlier statement that `keepassxc-proxy-lib` is "used by
  `keepassxc-client`" was wrong. They are independent implementations.
- **Correction:** an earlier `which keepassxc` returned "not found" because it searched `PATH`
  only. KeePassXC is installed at `/Applications/KeePassXC.app` and its browser socket exists.
- Noted that the identification key's secret half is never used on the wire (Option D).
- Flagged unverified assumptions: custom-scheme URL matching, confirmation-dialog behaviour,
  macOS behaviour of both crates.
- The commit of the integration-test files was requested but declined twice at the `git commit`
  permission prompt (`git commit*` is `ask` in `~/.config/opencode/opencode.jsonc`). The files
  remain staged.
