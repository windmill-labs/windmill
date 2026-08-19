import DOMPurify from 'dompurify'
import { SettingService } from '$lib/gen'
import { DEFAULT_HUB_BASE_URL } from '$lib/hub'
import type { ImportProjectSummary } from '$lib/components/ImportProjectCard.svelte'

/**
 * The browser-reachable hub. `hub_accessible_url` exists precisely for this: on a
 * private instance `hub_base_url` may be an address only the server can resolve.
 * The (logged) layout does the same lookup, but the import wizard renders outside
 * it, so it has to ask for itself.
 */
export async function hubBrowserUrl(): Promise<string> {
	try {
		const accessible = (await SettingService.getGlobal({ key: 'hub_accessible_url' })) as string
		if (accessible) return accessible.replace(/\/+$/, '')
		const base = (await SettingService.getGlobal({ key: 'hub_base_url' })) as string
		if (base) return base.replace(/\/+$/, '')
	} catch {
		// Unset or unreadable — the public hub is the right default either way.
	}
	return DEFAULT_HUB_BASE_URL.replace(/\/+$/, '')
}

/** Shape of `GET <hub>/projects/<slug>` — the hub's own summary endpoint. */
interface HubProject {
	slug: string
	name: string
	summary: string
	author: string
	apps: string[]
	logoApp: string | null
	hasLogo: boolean
	counts: { scripts: number; flows: number; apps: number; resources: number; total: number }
}

/**
 * Fetches one project's presentation straight from the hub. Cross-origin and
 * unauthenticated by design: this runs before the wizard has a workspace, so the
 * workspace-scoped `/api/w/<ws>/hub/...` proxy is not available yet. The endpoint
 * is public and sends `Access-Control-Allow-Origin: *`.
 */
export async function fetchHubProject(slug: string): Promise<ImportProjectSummary> {
	const hub = await hubBrowserUrl()
	const res = await fetch(`${hub}/projects/${encodeURIComponent(slug)}`, {
		headers: { accept: 'application/json' }
	})
	if (!res.ok) throw new Error(`hub returned ${res.status}`)
	const p = (await res.json()) as HubProject
	return {
		slug: p.slug,
		name: p.name,
		summary: p.summary,
		author: p.author,
		apps: p.apps ?? [],
		// A project with an uploaded logo shows that; otherwise the icon of the
		// integration it is filed under, otherwise its first integration.
		logoUrl: p.hasLogo ? `${hub}/projects/${encodeURIComponent(p.slug)}/logo` : undefined,
		iconApps: [p.logoApp, ...(p.apps ?? [])].filter(
			(a, i, all): a is string => !!a && all.indexOf(a) === i
		),
		hub,
		counts: {
			apps: p.counts?.apps ?? 0,
			flows: p.counts?.flows ?? 0,
			scripts: p.counts?.scripts ?? 0,
			resources: p.counts?.resources ?? 0
		}
	}
}

/**
 * One integration's icon, as SVG markup rather than an <img>: the hub's icons are
 * `fill="currentColor"`, so inlining them lets the icon follow the page's theme.
 * Returns undefined when the hub ships no icon for that slug (it 404s), which is
 * the caller's cue to fall back to a placeholder.
 *
 * Sanitized before it is returned, because the markup is inlined into this
 * authenticated origin and the hub is not necessarily ours: `hub_base_url` is an
 * instance setting, and the import wizard can be pointed at any hub by URL. A
 * hostile or compromised one answering with `<svg onload=…>` would otherwise run
 * script here. SVG profile only — no HTML, and `svg` plus `svgFilters` namespaces.
 */
export async function fetchAppIcon(hub: string, app: string): Promise<string | undefined> {
	try {
		const res = await fetch(`${hub}/icons/integrations/${encodeURIComponent(app)}.svg`)
		if (!res.ok) return undefined
		const raw = (await res.text()).trim()
		if (!raw.startsWith('<svg')) return undefined
		const clean = DOMPurify.sanitize(raw, { USE_PROFILES: { svg: true, svgFilters: true } }).trim()
		return clean.startsWith('<svg') ? clean : undefined
	} catch {
		return undefined
	}
}
