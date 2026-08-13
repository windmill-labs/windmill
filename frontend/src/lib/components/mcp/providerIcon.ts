import type { Component } from 'svelte'
import { MCP_REGISTRY } from './registry'

/**
 * The provider icon for a server url, loaded on demand.
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

function hostnameOf(url: string): string | undefined {
	try {
		return new URL(url).hostname
	} catch {
		return undefined
	}
}

/** `mcp.notion.com` names notion; the transport labels say nothing about who it is. */
function providerLabel(hostname: string): string | undefined {
	const labels = hostname.split('.').filter((l) => !['www', 'mcp', 'api', 'app'].includes(l))
	const name = labels[0]?.toLowerCase().replace(/[^a-z0-9]/g, '')
	return name && name !== 'localhost' ? name : undefined
}

const cache = new Map<string, Component<any> | undefined>()

export async function loadProviderIcon(url: unknown): Promise<Component<any> | undefined> {
	if (typeof url !== 'string') return undefined
	const hostname = hostnameOf(url)
	if (!hostname) return undefined

	// A registry server carries its own icon, and its host does not always name
	// it: github's mcp server answers on api.githubcopilot.com.
	const known = MCP_REGISTRY.find((e) => hostnameOf(e.url) === hostname)
	if (known) return known.icon

	const name = providerLabel(hostname)
	if (!name) return undefined
	if (cache.has(name)) return cache.get(name)

	// Both shapes exist in the icon folder (`NotionIcon.svelte`, `Slack.svelte`).
	const match = Object.keys(iconModules).find((path) => {
		const file = path.split('/').pop()?.replace('.svelte', '').toLowerCase()
		return file === `${name}icon` || file === name
	})
	let icon: Component<any> | undefined
	if (match) {
		try {
			icon = (await iconModules[match]()).default
		} catch {
			icon = undefined
		}
	}
	cache.set(name, icon)
	return icon
}
