# fnox-plugin-keepassxc

## Goal

Make tokens stored in KeePassXC available to programs as environment variables,
through [fnox](https://fnox.jdx.dev) and [mise](https://mise.jdx.dev), using a
running instance of KeePassXC.

The plugin is an fnox `lua` provider. It reads individual entries from a
**running, unlocked** KeePassXC instance through its Browser Integration
protocol. Only entries explicitly tagged for exposure, through a
`fnox://<name>` URL, are reachable.

## Status

Design and exploration stage.

Working:

* The Lua provider maps the client's exit codes to fnox behaviour. Exit 4
  calls `fnox.auth_error` when the database is locked or not associated.
* The Rust client parses `fnox://<name>` references, maps errors to exit
  codes, and implements a lookup against KeePassXC through the
  `keepassxc-client` crate. The lookup is tested in-process against a fake
  KeePassXC.
* Live `fnox://` lookups against KeePassXC 2.8.0 were checked on a dedicated
  test database.

Not yet done:

* The client binary still echoes the parsed name. It is not wired to real
  lookups or to `associate`.
* Where the association is stored (macOS Keychain or a `0600` file) is
  undecided.

See [doc/project-plan.md](doc/project-plan.md) for the plan, decision log, and
open questions.

## Design rationale

### Why not the `keepass` provider?

fnox's built-in `keepass` provider reads the `.kdbx` file directly. That suits
scripts, CI, and any setup without a desktop session: it works headless and
needs only the database file and its password. For interactive use, it needs
the master password from an environment variable or a prompt, decrypts the
whole database in the fnox process, and cannot reuse an unlock made in the
KeePassXC GUI, such as Touch ID or a YubiKey.

This project uses the KeePassXC instance that is already running instead:

* The master password stays in KeePassXC.
* The database is decrypted only inside KeePassXC.
* An unlock made in the KeePassXC GUI, such as Touch ID or a YubiKey, is
  reused.
* Only entries tagged with a `fnox://<name>` URL are reachable, and KeePassXC
  can ask for confirmation per entry.

The trade-off is that the protocol needs a running, unlocked KeePassXC, so
headless and CI use is not supported. Use the built-in provider for those. The
same reasoning applies to `keepassxc-cli`, which also decrypts the `.kdbx` file
directly.

### How it works

* fnox Lua plugins cannot open sockets or perform NaCl crypto, so a small Rust
  client, `kpxc-client`, speaks the protocol. `provider.lua` runs it through
  `fnox.cmd`.
* The protocol is handled by `keepassxc-client`, pinned exactly. This project
  owns no protocol code.

## Layout

* `provider.lua` — the fnox plugin entrypoint (must stay at repo root so
  `owner/repo` git installs work; see fnox's [Lua plugin
  docs](https://fnox.jdx.dev/providers/lua.html)).
* `client/` — a Rust CLI built on `keepassxc-client`. `provider.lua` invokes
  it via `fnox.cmd`.

## Development

Tool versions are pinned via `mise` (see `mise.toml`).

```sh
mise install

# Rust
cd client && cargo test

# Lua (uses a project-local LuaRocks tree for dev-only deps)
mise exec -- luarocks --tree=lua_modules install busted   # once
./scripts/test-lua.sh
```
