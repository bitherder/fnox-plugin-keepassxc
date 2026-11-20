# fnox-plugin-keepassxc

An [fnox](https://fnox.jdx.dev) `lua` provider that reads secrets from a
**running, unlocked KeePassXC** instance via its Browser Integration protocol,
instead of decrypting a `.kdbx` file directly. The master password never
passes through fnox or this plugin; only entries explicitly tagged for
exposure (via a `fnox://<name>` URL) are reachable.

## Layout

- `provider.lua` — the fnox plugin entrypoint (must stay at repo root so
  `owner/repo` git installs work; see fnox's [Lua plugin
  docs](https://fnox.jdx.dev/providers/lua.html)).
- `client/` — a Rust CLI that speaks the KeePassXC Browser Protocol
  (NaCl-box encrypted messages over KeePassXC's local socket) and is invoked
  by `provider.lua` via `fnox.cmd`.

## Status

Design/exploration stage. See [doc/project-plan.md](doc/project-plan.md) for
the plan, decision log, and options under consideration.

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
