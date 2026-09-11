import type { Component } from 'svelte'
import { findMcpEntry, findMcpEntryByUrl } from './registry'

/**
 * Provider icons for connected servers, resolved in two halves so a cached key
 * is enough to draw one: `providerKey` needs the server url (one read per
 * resource, since the list endpoint strips values), `loadProviderIcon` needs
 * only the key.
 *
 * Windmill ships an icon per integration, but importing them through
 * `appIconComponent` would pull all ~230 into whatever chunk asks for one, and
 * the chat does not otherwise reach that barrel. `import.meta.glob` gives the
 * filenames at build time and the module only when a match is actually used, so
 * a workspace with two connections downloads two icons.
 */
const iconModules = import.meta.glob('$lib/components/icons/*.svelte') as Record<
	string,
	() => Promise<{ default: Component<any> }>
>

/**
 * Who a server url belongs to, as a stable string worth caching.
 *
 * Registry servers answer with their entry id, because their host does not
 * always name them: github's mcp server answers on api.githubcopilot.com.
 * Anything else is named by its host, since `mcp.notion.com` says notion while
 * the transport labels say nothing about who it is.
 */
export function providerKey(url: unknown): string | undefined {
	if (typeof url !== 'string') return undefined
	const known = findMcpEntryByUrl(url)
	if (known) return known.id
	let hostname: string
	try {
		hostname = new URL(url).hostname
	} catch {
		return undefined
	}
	const labels = hostname.split('.').filter((l) => !['www', 'mcp', 'api', 'app'].includes(l))
	const name = labels[0]?.toLowerCase().replace(/[^a-z0-9]/g, '')
	// An address names nobody, and a self-hosted server on one would otherwise be
	// cached under a key like `127`.
	if (!name || name === 'localhost' || /^\d+$/.test(name)) return undefined
	return name
}

const cache = new Map<string, Component<any> | undefined>()

export async function loadProviderIcon(
	key: string | undefined | null
): Promise<Component<any> | undefined> {
	if (!key) return undefined
	const entry = findMcpEntry(key)
	if (entry) return entry.icon
	if (cache.has(key)) return cache.get(key)

	// Both shapes exist in the icon folder (`NotionIcon.svelte`, `Slack.svelte`).
	const match = Object.keys(iconModules).find((path) => {
		const file = path.split('/').pop()?.replace('.svelte', '').toLowerCase()
		return file === `${key}icon` || file === key
	})
	let icon: Component<any> | undefined
	if (match) {
		try {
			icon = (await iconModules[match]()).default
		} catch {
			icon = undefined
		}
	}
	cache.set(key, icon)
	return icon
}
