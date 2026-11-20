#!/usr/bin/env bash
# Runs the Lua test suite. Requires `luarocks install --tree=lua_modules busted` once.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
export LUA_PATH="./lua_modules/share/lua/5.4/?.lua;./lua_modules/share/lua/5.4/?/init.lua;;"
export LUA_CPATH="./lua_modules/lib/lua/5.4/?.so;;"
exec mise exec -- busted --exclude-tags=integration "$@"
