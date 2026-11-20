-- fnox plugin entrypoint. Must stay at repo root: fnox requires
-- `provider.lua` at the top of a plugin directory for git-installed plugins.
--
-- Reads secrets from a running, unlocked KeePassXC instance by shelling out
-- to the `kpxc-client` binary (see client/), which speaks the KeePassXC
-- Browser Integration protocol directly. The master password never passes
-- through fnox or this plugin.

local cmd = require("fnox.cmd")

local M = {}

-- Resolves ctx.options.client_bin, defaulting to a bare command name so it
-- just needs to be on PATH.
local function client_bin(ctx)
	return (ctx.options and ctx.options.client_bin) or "kpxc-client"
end

function M.get(ctx, ref)
	local result = cmd.run({ client_bin(ctx), ref })
	if result.ok then
		return (result.stdout:gsub("\n$", ""))
	end
	if result.status == 1 or result.status == 2 then
		error(result.stderr)
	end
	if result.status == 4 then
		require("fnox").auth_error(result.stderr)
		return nil
	end
	if result.status == 5 then
		error("KeePassXC is not reachable: " .. result.stderr)
	end
	return nil
end

function M.test(ctx)
	cmd.exec({ client_bin(ctx), "--version" })
end

return M
