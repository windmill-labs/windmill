import { enabledMcpPaths } from '$lib/components/mcp/enabledServers'
import { listVisibleMcpServers } from '$lib/components/mcp/ownServers'

const TURN_ON =
	'The user turns servers on in the chat settings, under MCP connections; the setting is kept per browser.'

/**
 * Why an MCP call cannot reach `path` (or any server, when the MCP tools are not
 * registered at all). Listed only on this error path, so a workspace without MCP
 * still pays nothing per iteration. Names only servers the user can see in MCP
 * connections, so it reveals nothing the settings page would not.
 */
export async function mcpUnavailableMessage(workspace: string, path?: string): Promise<string> {
	let visible: string[]
	try {
		visible = (await listVisibleMcpServers(workspace)).map((s) => s.path)
	} catch (e: any) {
		return `Could not list the MCP servers: ${e?.body?.error?.message ?? e?.body ?? e?.message ?? String(e)}`
	}
	const enabled = new Set(enabledMcpPaths(workspace))
	const on = visible.filter((p) => enabled.has(p))
	const off = visible.filter((p) => !enabled.has(p))

	if (path && off.includes(path)) {
		return `MCP server "${path}" is connected but turned off. ${TURN_ON}`
	}
	// A server that is on yet has no tools registered means the chat is not in global mode.
	if (path ? on.includes(path) : on.length > 0) {
		return 'MCP tools are only available in global mode. Use change_mode to switch.'
	}
	if (visible.length === 0) {
		return 'No MCP server is connected in this workspace. The user connects one in the chat settings, under MCP connections.'
	}
	return [
		path
			? `No MCP server "${path}" is connected in this workspace.`
			: 'No MCP server is turned on.',
		on.length > 0 ? `Turned on: ${on.join(', ')}.` : '',
		off.length > 0 ? `Connected but turned off: ${off.join(', ')}. ${TURN_ON}` : ''
	]
		.filter(Boolean)
		.join(' ')
}
