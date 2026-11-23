-- fnox plugin entrypoint. Must stay at repo root: fnox requires
-- `provider.lua` at the top of a plugin directory for git-installed plugins.
--
-- Reads secrets from a running, unlocked KeePassXC instance by shelling out
-- to the `kpxc-client` binary (see client/), which speaks the KeePassXC
-- Browser Integration protocol directly. The master password never passes
-- through fnox or this plugin.

local cmd = require("fnox.cmd")

local M = {}

-- Prefer configured client, then plugin-local binary, then PATH.
local function client_bin(ctx)
	local configured = ctx.options and ctx.options.client_bin
	if configured then
		return configured
	end

	if ctx.plugin_dir then
		local separator = package.config and package.config:sub(1, 1) or "/"
		local suffix = ctx.plugin_dir:sub(-1) == separator and "" or separator
		local path = ctx.plugin_dir .. suffix .. "client" .. separator .. "target"
			.. separator .. "release" .. separator .. "kpxc-client"
		local file = io.open(path, "rb")
		if file then
			file:close()
			return path
		end
	end

	return "kpxc-client"
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
