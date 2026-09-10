import type { Component } from 'svelte'
import { ResourceService } from '$lib/gen'
import { cachedProviderMark } from './iconCache'
import { loadProviderIcon, providerKey } from './providerIcon'

/** Windmill's own icon for a connected server's integration, when it ships one. The
 * server's published icon is preferred over this and is resolved separately. */
export type McpServerMark = { icon?: Component<any> }

// One resolution per server per session, shared by every transcript row naming it —
// a chat can hold dozens of calls against the same server.
const marks = new Map<string, Promise<McpServerMark>>()

export function resolveMcpServerMark(workspace: string, path: string): Promise<McpServerMark> {
	const key = `${workspace}:${path}`
	let pending = marks.get(key)
	if (!pending) {
		pending = load(workspace, path)
		marks.set(key, pending)
	}
	return pending
}

async function load(workspace: string, path: string): Promise<McpServerMark> {
	const cached = cachedProviderMark(workspace, path)
	if (cached) return { icon: await loadProviderIcon(cached.key) }
	try {
		// Deliberately not written back to the shared cache: that entry is keyed by
		// `editedAt` for the server list's sake, and storing one from here — where the
		// row is a past call and `editedAt` is unknown — would make every list re-read.
		const resource = await ResourceService.getResource({ workspace, path })
		const url = (resource.value as { url?: unknown } | undefined)?.url
		return { icon: await loadProviderIcon(providerKey(url)) }
	} catch {
		return {}
	}
}
