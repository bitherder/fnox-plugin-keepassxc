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

Prototype, pre-release. Association and lookups have been run live against
KeePassXC. The provider passes `fnox provider test` using a local plugin
directory and its plugin-local release binary. The user also reports that
`fnox get` retrieved the test-entry value and successfully ran `auth_command`
to create a fresh association. A mise task also received the test value, checked
it was nonempty, and did not print it. With `FNOX_DAEMON=on`, the test task ran
without a KeePassXC prompt; with it off, the task prompted. The current mise
build has a protocol mismatch for its direct socket path, so this project accepts
the fnox CLI fallback and its daemon cache. Use by a real application and
`fnox exec` remain untested.

Working:

* The Lua provider maps the client's exit codes to fnox behaviour. Exit 4
  calls `fnox.auth_error` when the database is locked or not associated.
* The Rust client has `associate` and `fnox://<name>` commands, built on the
  `keepassxc-client` crate. The association is stored in a `0600` file under
  `$XDG_DATA_HOME`. Exit codes cover every failure path, and the lookup is
  tested in-process against a fake KeePassXC.
* On 2026-11-23 the owner ran `associate`, approved it in KeePassXC, and ran
  `fnox://` lookups for a test entry and a real token entry. Secret output was
  not recorded here.

Not yet done:

* Tagging every token entry with a `fnox://` URL.
* Testing the value with a real application and through `fnox exec`.
* Choosing the shared fnox plugin source and how resolved values reach the
  environment.
* Release tags and published binaries. See [Releases](#releases).

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

### Passing secrets to programs

Run a command with fnox-resolved secrets using `fnox exec -- <command>`. mise can also grant secrets
to individual tasks through its experimental
[`secrets.fnox` integration](https://mise.jdx.dev/environments/secrets/fnox.html). That integration
does not put secrets in the activated shell or `mise env`; each task must list the keys it needs.
See fnox's [mise guide](https://fnox.jdx.dev/guide/mise-integration.html).

fnox's optional [daemon](https://fnox.jdx.dev/guide/daemon.html) can cache resolved
secrets in memory. It is disabled by default. With it enabled, first lookup still
resolves through the provider and may prompt in KeePassXC; subsequent reads can
use the cache until it expires or the daemon stops. The daemon does not write
cached values to disk, but other processes running as the same user can request
them. mise reads directly from the daemon only on a cache hit; on a miss it asks
fnox to resolve the value.

The locally installed mise 2026.10.17 embeds `fnox-client` 1.40.0 (wire
protocol 6), while fnox 1.50.0 uses protocol 8. The socket name includes the
protocol version, so direct socket access is mismatched. The tested CLI fallback
still uses fnox's daemon cache; no downgrade or socket-path override is needed.

Mise does not connect stdin to a task that receives secrets by default. If the
task needs interactive stdin, set `interactive = true`; its output then goes
directly to the terminal and is not redacted. Avoid that setting for tasks that
do not need terminal input.

## Security

`kpxc-client associate` stores the association in
`$XDG_DATA_HOME/fnox-plugin-keepassxc/association.json`. If `XDG_DATA_HOME` is
unset or relative, the path falls back to
`~/.local/share/fnox-plugin-keepassxc/association.json`. The app's directory is
created with mode `0700`, and the file with `0600`.

`XDG_DATA_HOME` must be absolute. For isolated tests, use an absolute temporary
directory outside the repository.

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

Tool versions are pinned in `mise.toml`. Run `mise install` once.

### Tasks

| Task                        | What it does                                                                    |
| --------------------------- | ------------------------------------------------------------------------------- |
| `mise run build`            | Builds the release binary into `client/target/release/kpxc-client`.             |
| `mise run test`             | Runs the Lua unit suite, the Rust tests, and the Lua-to-Rust integration suite. |
| `mise run test:lua`         | Runs the Lua unit suite in `spec/` with busted.                                 |
| `mise run test:rust`        | Runs the Rust unit and integration tests with `cargo test`.                     |
| `mise run test:integration` | Builds the debug binary and runs the Lua-to-Rust integration suite.             |

The Lua suites need busted in the project-local LuaRocks tree. Install it once:

```sh
mise exec -- luarocks --tree=lua_modules install busted
```

The integration suite runs `client/target/debug/kpxc-client` by default. Set
`KPXC_CLIENT_BIN` to use a different binary. It runs with temporary `HOME` and
`XDG_DATA_HOME` directories, so it never reads your association file.

### Building the client

`mise run build` writes the release binary to `client/target/release/`, which
is git-ignored. `provider.lua` resolves the executable in this order:

1. `options.client_bin`, if set.
2. `<ctx.plugin_dir>/client/target/release/kpxc-client`, if that file exists.
3. Bare `kpxc-client` on `PATH`.

This was verified with fnox 1.50.0 using the repo as a local Lua plugin: the
provider found the release binary without a global install or PATH entry. To
override the discovered binary, set `client_bin`:

```toml
[providers.keepassxc]
type = "lua"
plugin = "keepassxc"
options = { client_bin = "/path/to/fnox-plugin-keepassxc/client/target/release/kpxc-client" }
auth_command = "/path/to/fnox-plugin-keepassxc/client/target/release/kpxc-client associate"
```

The `auth_command` is run by fnox outside the plugin, so it still needs an
explicit path or a binary on PATH.

### Releases

No releases are published yet. The repository has no git tags, and no release
binaries are attached anywhere. Release binaries are deferred (plan decision D12).

fnox does not build git-installed plugins. A user installing from git must run
`mise run build` in the plugin directory; the provider then finds the binary
relative to `ctx.plugin_dir`. `auth_command` still needs an explicit path or a
binary on PATH.

The intended release procedure is manual, and it has not been exercised yet:

1. Run `mise run test` and check that everything passes.
2. Update `version` in `client/Cargo.toml`.
3. Run `mise run build`, and check that `client/target/release/kpxc-client --version`
   prints the new version.
4. Commit the version change, then tag that commit `vX.Y.Z`.
5. Push the commit and the tag.
6. Users pin the tag in `[plugins]`, for example
   `keepassxc = "bitherder/fnox-plugin-keepassxc#vX.Y.Z"`.
