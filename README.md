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
* The Rust client has `associate` and `fnox://<name>` commands, built on the
  `keepassxc-client` crate. The association is stored in a `0600` file under
  `$XDG_DATA_HOME`. Exit codes cover every failure path, and the lookup is
  tested in-process against a fake KeePassXC.
* Live `fnox://` lookups against KeePassXC 2.8.0 were checked on a dedicated
  test database. Those checks predate the CLI and used a throwaway crate.

Not yet done:

* The CLI has not yet been run live against KeePassXC: `associate` and a
  `fnox://` lookup through the built binary.

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

## Security

`kpxc-client associate` stores the association in
`$XDG_DATA_HOME/fnox-plugin-keepassxc/association.json`. If `XDG_DATA_HOME` is
unset, the path is `~/.local/share/fnox-plugin-keepassxc/association.json`. The
app's directory is created with mode `0700`, and the file with `0600`.

The file holds the association ID and the 32-byte identification key. Treat it as
a credential. Any process running as your user can read it and ask a running,
unlocked KeePassXC for entries tagged with `fnox://`. It does not expose the
master password, or any entry without a `fnox://` URL. Unlike a macOS Keychain
item, the file has no OS-level access control, so protection depends on the file
permissions and on KeePassXC's per-entry approval.

Precautions:

* Keep KeePassXC locked when you are not using it. Lookups need it unlocked.
* Leave the **Remember** checkbox in KeePassXC's access dialog unticked, so each
  lookup asks for approval. Ticking it appears to save the approval, so later
  lookups for that entry would not prompt. The source reading suggests this, but
  it has not been checked live yet.
* Tag only the entries you need, and prefer dedicated entries for fnox.
* Do not put the file under version control, in a sync folder, or in a backup you
  share. Your dotfiles repository must not track it.
* To revoke access, delete the file and remove the association from KeePassXC's
  Browser Integration settings. I have not yet confirmed the location of that
  setting in your KeePassXC version.

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
