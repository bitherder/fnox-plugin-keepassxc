-- Integration test: spawns the real compiled kpxc-client binary through a
-- real (non-mocked) fnox.cmd shim, proving provider.lua's get() hook calls
-- through to the Rust client and returns its output.
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

	it("calls the real client binary and returns its stdout", function()
		local provider = load_provider()

		local value = provider.get({ options = { client_bin = client_bin } }, "fnox://github-token")

		assert.equal("github-token", value)
	end)

	it("raises on a reference the client rejects", function()
		local provider = load_provider()

		assert.has_error(function()
			provider.get({ options = { client_bin = client_bin } }, "not-a-valid-reference")
		end)
	end)
end)
