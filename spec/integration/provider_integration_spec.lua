-- Integration test: spawns the real compiled kpxc-client binary through a
-- real (non-mocked) fnox.cmd shim, proving provider.lua's get() hook calls
-- through to the Rust client and maps its exit codes.
--
-- The binary runs with an isolated, empty data directory (set up by
-- scripts/test-integration.sh), so no association is stored. The client
-- must report that as an auth error (exit 4), which get() forwards to
-- fnox's auth_error hook.
--
-- Requires the binary to be built first: `cargo build --manifest-path
-- client/Cargo.toml`. Run via scripts/test-integration.sh, or directly with
-- `busted --tags=integration spec/integration`.
--
-- Excluded from the default `busted` run (see .busted's default exclude)
-- since it needs a build artifact rather than just source.

require("spec.support.real_fnox_cmd")

describe("provider.get (integration) #integration", function()
	local client_bin = os.getenv("KPXC_CLIENT_BIN") or "client/target/debug/kpxc-client"

	local function load_provider()
		package.loaded["provider"] = nil
		return require("provider")
	end

	after_each(function()
		package.loaded["fnox"] = nil
	end)

	it("reports an authentication error when the client has no association", function()
		local auth_message
		package.loaded["fnox"] = {
			auth_error = function(message)
				auth_message = message
			end,
		}
		local provider = load_provider()

		local value = provider.get({ options = { client_bin = client_bin } }, "fnox://github-token")

		assert.is_string(auth_message)
		assert.truthy(auth_message:find("associate", 1, true))
		assert.is_nil(value)
	end)

	it("raises on a reference the client rejects", function()
		local provider = load_provider()

		assert.has_error(function()
			provider.get({ options = { client_bin = client_bin } }, "not-a-valid-reference")
		end)
	end)
end)
