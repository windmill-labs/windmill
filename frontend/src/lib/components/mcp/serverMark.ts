import type { Component } from 'svelte'
import { ResourceService } from '$lib/gen'
import { cachedProviderMark } from './iconCache'
import { loadProviderIcon, providerKey } from './providerIcon'

/** Windmill's own icon for a connected server's integration, when it ships one. */
export type McpServerMark = { icon?: Component<any> }

// One resolution per server per session, shared by every transcript row naming it —
// a chat can hold dozens of calls against the same server.
const marks = new Map<string, Promise<McpServerMark>>()

export function resolveMcpServerMark(workspace: string, path: string): Promise<McpServerMark> {
	const key = `${workspace}:${path}`
	let pending = marks.get(key)
	if (!pending) {
		// A failed read is dropped rather than memoized: one offline blip or a 401 during
		// a token refresh would otherwise leave the server unmarked in every later row
		// until the page reloads.
		pending = load(workspace, path).catch(() => {
			marks.delete(key)
			return {}
		})
		marks.set(key, pending)
	}
	return pending
}

/**
 * Forget what was resolved, for the settings section to call when it reloads the
 * connections: a path can be reconnected to a different provider, and a mark held for
 * the life of the page would go on marking new call rows with the old provider's icon.
 */
export function forgetMcpServerMarks() {
	marks.clear()
}

async function load(workspace: string, path: string): Promise<McpServerMark> {
	const cached = cachedProviderMark(workspace, path)
	if (cached) return { icon: await loadProviderIcon(cached.key) }
	// Deliberately not written back to the shared cache: that entry is keyed by
	// `editedAt` for the server list's sake, and storing one from here — where the
	// row is a past call and `editedAt` is unknown — would make every list re-read.
	const resource = await ResourceService.getResource({ workspace, path })
	const url = (resource.value as { url?: unknown } | undefined)?.url
	return { icon: await loadProviderIcon(providerKey(url)) }
}
