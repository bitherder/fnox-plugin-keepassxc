local fnox_mock = require("spec.fnox_mock")

describe("provider.get", function()
	-- reload provider.lua fresh each time so module-level requires re-resolve
	-- against the current mock state
	local function load_provider()
		package.loaded["provider"] = nil
		return require("provider")
	end

	after_each(function()
		fnox_mock.run_result = nil
		fnox_mock.last_run_args = nil
	end)

	it("returns trimmed stdout from the client binary on success", function()
		fnox_mock.run_result = { ok = true, stdout = "s3cr3t\n", stderr = "" }
		local provider = load_provider()

		local value = provider.get({ options = {} }, "fnox://github-token")

		assert.equal("s3cr3t", value)
		assert.same({ "kpxc-client", "fnox://github-token" }, fnox_mock.last_run_args.command)
	end)

	it("uses options.client_bin when set", function()
		fnox_mock.run_result = { ok = true, stdout = "x\n", stderr = "" }
		local provider = load_provider()

		provider.get({ options = { client_bin = "./target/debug/kpxc-client" } }, "fnox://x")

		assert.equal("./target/debug/kpxc-client", fnox_mock.last_run_args.command[1])
	end)

	it("returns nil when the entry is not found", function()
		fnox_mock.run_result = { ok = false, stdout = "", stderr = "not found" }
		local provider = load_provider()

		local value = provider.get({ options = {} }, "fnox://missing")

		assert.is_nil(value)
	end)

	it("raises on an invalid reference", function()
		fnox_mock.run_result = { ok = false, stdout = "", stderr = "invalid reference: bogus" }
		local provider = load_provider()

		assert.has_error(function()
			provider.get({ options = {} }, "bogus")
		end)
	end)
end)
