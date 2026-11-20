local fnox_mock = require("spec.fnox_mock")

describe("provider.get", function()
	-- reload provider.lua fresh each time so module-level requires re-resolve
	-- against the current mock state
	local function load_provider()
		package.loaded["provider"] = nil
		return require("provider")
	end

	-- Returns the message of the error raised by fn, failing if nothing raised.
	local function raised_message(fn)
		local ok, err = pcall(fn)
		assert.is_false(ok, "expected provider.get to raise an error")
		return tostring(err)
	end

	after_each(function()
		fnox_mock.run_result = nil
		fnox_mock.last_run_args = nil
		fnox_mock.last_auth_error = nil
	end)

	it("returns trimmed stdout from the client binary on success", function()
		fnox_mock.run_result = { status = 0, ok = true, stdout = "s3cr3t\n", stderr = "" }
		local provider = load_provider()

		local value = provider.get({ options = {} }, "fnox://github-token")

		assert.equal("s3cr3t", value)
		assert.same({ "kpxc-client", "fnox://github-token" }, fnox_mock.last_run_args.command)
	end)

	it("uses options.client_bin when set", function()
		fnox_mock.run_result = { status = 0, ok = true, stdout = "x\n", stderr = "" }
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

	it("raises with the client's stderr on an invalid reference (exit 2)", function()
		fnox_mock.run_result = {
			status = 2,
			ok = false,
			stdout = "",
			stderr = "invalid reference: bogus (expected fnox://<name>)",
		}
		local provider = load_provider()

		local message = raised_message(function()
			provider.get({ options = {} }, "bogus")
		end)

		assert.truthy(message:find("invalid reference: bogus (expected fnox://<name>)", 1, true))
	end)

	it("raises with the client's stderr on a general failure (exit 1)", function()
		fnox_mock.run_result = { status = 1, ok = false, stdout = "", stderr = "boom: unexpected reply" }
		local provider = load_provider()

		local message = raised_message(function()
			provider.get({ options = {} }, "fnox://github-token")
		end)

		assert.truthy(message:find("boom: unexpected reply", 1, true))
	end)

	it("hands off to fnox.auth_error instead of raising when auth is required (exit 4)", function()
		fnox_mock.run_result = { status = 4, ok = false, stdout = "", stderr = "database is locked" }
		local provider = load_provider()

		local value = provider.get({ options = {} }, "fnox://github-token")

		assert.is_nil(value)
		assert.is_not_nil(fnox_mock.last_auth_error, "expected fnox.auth_error to be called")
		assert.truthy(fnox_mock.last_auth_error:find("database is locked", 1, true))
	end)

	it("raises that KeePassXC is not reachable, with stderr, when unavailable (exit 5)", function()
		fnox_mock.run_result = {
			status = 5,
			ok = false,
			stdout = "",
			stderr = "cannot connect to KeePassXC socket",
		}
		local provider = load_provider()

		local message = raised_message(function()
			provider.get({ options = {} }, "fnox://github-token")
		end)

		assert.truthy(message:find("KeePassXC is not reachable", 1, true))
		assert.truthy(message:find("cannot connect to KeePassXC socket", 1, true))
	end)
end)
