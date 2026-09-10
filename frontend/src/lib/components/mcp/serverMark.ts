import type { Component } from 'svelte'
import { ResourceService } from '$lib/gen'
import { cachedProviderMark, providerHost } from './iconCache'
import { loadProviderIcon, providerKey } from './providerIcon'

/** What identifies a connected server visually: Windmill's icon for that integration
 * if it ships one, otherwise the host its favicon can be fetched from. */
export type McpServerMark = { icon?: Component<any>; host?: string }

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
	if (cached) return { icon: await loadProviderIcon(cached.key), host: cached.host }
	try {
		// Deliberately not written back to the shared cache: that entry is keyed by
		// `editedAt` for the server list's sake, and storing one from here — where the
		// row is a past call and `editedAt` is unknown — would make every list re-read.
		const resource = await ResourceService.getResource({ workspace, path })
		const url = (resource.value as { url?: unknown } | undefined)?.url
		return { icon: await loadProviderIcon(providerKey(url)), host: providerHost(url) }
	} catch {
		return {}
	}
}
