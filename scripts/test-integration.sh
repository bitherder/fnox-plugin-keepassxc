#!/usr/bin/env bash
# Builds the real client binary and runs the Lua<->Rust integration suite
# against it (tests tagged #integration, excluded from the default unit run).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

mise exec -- cargo build --manifest-path client/Cargo.toml

export LUA_PATH="./lua_modules/share/lua/5.4/?.lua;./lua_modules/share/lua/5.4/?/init.lua;;"
export LUA_CPATH="./lua_modules/lib/lua/5.4/?.so;;"
exec mise exec -- busted --tags=integration spec/integration "$@"
