-- Minimal stand-ins for the `fnox.*` modules that only exist inside the real
-- fnox runtime, so provider.lua can be unit tested standalone with busted.
--
-- Each test should set `fnox_cmd_mock.run_result` / `.exec_result` (or
-- `.exec_error`) before calling into provider.lua, then reset them in an
-- `after_each`.

local M = {
	run_result = nil, -- { status, ok, stdout, stderr }
	last_run_args = nil,
	exec_error = nil,
	last_exec_args = nil,
	last_auth_error = nil,
}

package.loaded["fnox.cmd"] = {
	run = function(command, opts)
		M.last_run_args = { command = command, opts = opts }
		return M.run_result
	end,
	exec = function(command, opts)
		M.last_exec_args = { command = command, opts = opts }
		if M.exec_error then
			error(M.exec_error)
		end
		return M.exec_result or ""
	end,
}

package.loaded["fnox"] = {
	auth_error = function(message)
		M.last_auth_error = message
	end,
}

return M
