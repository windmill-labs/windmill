/**
 * Which MCP servers the chat may use, per workspace.
 *
 * Being able to read an `mcp` resource is not the same as wanting the chat to
 * act through it: a resource in a shared folder is readable by a whole team, and
 * each server's tools both reach an external system and put their descriptions
 * in the model's context. So a server is off until it is turned on here, and
 * connecting one through the chat turns it on for the person who connected it.
 *
 * Stored per browser, like the chat's other per-user preferences.
 */
const KEY = 'wm_mcp_enabled'

function read(): Record<string, string[]> {
	if (typeof localStorage === 'undefined') return {}
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '{}')
	} catch {
		return {}
	}
}

function write(all: Record<string, string[]>) {
	try {
		localStorage.setItem(KEY, JSON.stringify(all))
	} catch (e) {
		console.error('Failed to persist enabled MCP servers', e)
	}
}

export function enabledMcpPaths(workspace: string): string[] {
	return read()[workspace] ?? []
}

export function isMcpEnabled(workspace: string, path: string): boolean {
	return enabledMcpPaths(workspace).includes(path)
}

export function setMcpEnabled(workspace: string, path: string, enabled: boolean) {
	const all = read()
	const current = new Set(all[workspace] ?? [])
	if (enabled) {
		current.add(path)
	} else {
		current.delete(path)
	}
	all[workspace] = [...current]
	write(all)
}
