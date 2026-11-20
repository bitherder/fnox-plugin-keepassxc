-- A real, minimal implementation of the `fnox.cmd` module's `run`/`exec`
-- surface, backed by `io.popen`, for integration tests that need
-- provider.lua to spawn an actual subprocess instead of a mock.
--
-- This is NOT a faithful reimplementation of fnox's module (no `env`/`cwd`
-- support, no argv-vs-shell-string distinction) — just enough to exercise
-- the real process boundary in tests.

local function shell_quote(s)
	return "'" .. s:gsub("'", "'\\''") .. "'"
end

local function to_shell_command(command)
	if type(command) == "string" then
		return command
	end
	local parts = {}
	for _, arg in ipairs(command) do
		parts[#parts + 1] = shell_quote(arg)
	end
	return table.concat(parts, " ")
end

local M = {}

function M.run(command)
	local shell_cmd = to_shell_command(command)
	local stderr_path = os.tmpname()

	local stdout_handle = io.popen(shell_cmd .. " 2>" .. shell_quote(stderr_path))
	local stdout = stdout_handle:read("*a")
	local ok, _, code = stdout_handle:close()

	local stderr = ""
	local stderr_handle = io.open(stderr_path)
	if stderr_handle then
		stderr = stderr_handle:read("*a")
		stderr_handle:close()
	end
	os.remove(stderr_path)

	return {
		status = code,
		ok = ok == true and code == 0,
		stdout = stdout,
		stderr = stderr,
	}
end

function M.exec(command)
	local result = M.run(command)
	if not result.ok then
		error(result.stderr)
	end
	return (result.stdout:gsub("\n$", ""))
end

package.loaded["fnox.cmd"] = M

return M
