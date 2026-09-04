import type { Component } from 'svelte'
import { appIconComponent } from '$lib/components/icons'
import { HubPublishService, SettingService } from '$lib/gen'
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
		counts: {
			apps: p.counts?.apps ?? 0,
			flows: p.counts?.flows ?? 0,
			scripts: p.counts?.scripts ?? 0,
			resources: p.counts?.resources ?? 0
		}
	}
}

/**
 * The icon for a hub integration slug, resolved from the icons Windmill already bundles.
 *
 * Not fetched from the hub: the hub renders these out of `@windmill-labs/components`, which
 * is this frontend's own package, so asking it over HTTP is a round trip to get our own
 * assets back — and it made the card depend on a cross-origin request that an `API_SECRET`
 * hub refuses anyway.
 *
 * The alias exists because the two repos disagree on one slug: the hub files Postgres scripts
 * under `postgres`, the icon set ships the mark as `postgresql`. The hub bridges it in
 * `aliasApp`; this is the same bridge on the consuming side.
 */
const HUB_APP_ICON_ALIAS: Record<string, string> = { postgres: 'postgresql' }

export function hubAppIcon(app: string): Component | undefined {
	return appIconComponent(HUB_APP_ICON_ALIAS[app] ?? app)
}

/** One row of the hub's catalogue (`GET <hub>/projects`), which carries no item counts. */
interface HubProjectListRow {
	slug: string
	name: string
	summary: string
	description: string
	readme: string
	author: string
	apps: string[]
	hasLogo: boolean
	stars: number
}

const DESCRIPTION_MAX = 320

/**
 * What a project says about itself, in prose.
 *
 * The hub's `description` field is empty on every published project — the writing all
 * goes in the readme — so the readme's opening paragraphs stand in. Everything from the
 * first heading onwards is dropped: that is the "Windmill concepts demonstrated" /
 * "Usage" material, which is documentation rather than a description. A readme that
 * *starts* with a heading (`## Description`) has it skipped rather than treated as the
 * end of the intro.
 */
export function hubProjectDescription(row: {
	description?: string
	readme?: string
	summary?: string
}): string {
	if (row.description?.trim()) return row.description.trim()

	const lines = (row.readme ?? '').split('\n')
	let i = 0
	while (i < lines.length && (lines[i].trim() === '' || lines[i].startsWith('#'))) i++
	const intro: string[] = []
	for (; i < lines.length; i++) {
		if (lines[i].startsWith('#')) break
		intro.push(lines[i])
	}

	const text = intro
		.join(' ')
		// Inline markdown only — the block syntax is already gone with the headings.
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/[*_`]/g, '')
		.replace(/\s+/g, ' ')
		.trim()
	if (!text) return row.summary?.trim() ?? ''
	if (text.length <= DESCRIPTION_MAX) return text
	// Cut on a word boundary: a description sliced mid-word reads as corrupted rather
	// than shortened.
	const cut = text.slice(0, DESCRIPTION_MAX)
	const lastSpace = cut.lastIndexOf(' ')
	return `${(lastSpace > DESCRIPTION_MAX * 0.6 ? cut.slice(0, lastSpace) : cut).trimEnd()}…`
}

/**
 * A card in the template picker. Everything it shows comes from the catalogue listing,
 * so a whole page of cards costs one request; the item counts, which only the import
 * step needs, are fetched per project by `fetchHubProject` when one is picked.
 * `id` is what `InfiniteList` dedupes rows by.
 */
export interface HubProjectPick {
	id: string
	slug: string
	name: string
	summary: string
	description: string
	author: string
	apps: string[]
	logoUrl?: string
	iconApps: string[]
	stars: number
}

let catalogue: { workspace: string; projects: Promise<HubProjectPick[]> } | undefined

/**
 * Every published project, most-starred first, fetched once per workspace and held for
 * the life of the page.
 *
 * Through the workspace-scoped proxy rather than straight at the hub the way
 * `fetchHubProject` goes: the catalogue endpoint sends no `Access-Control-Allow-Origin`,
 * so the browser cannot read it directly.
 */
export function hubProjectCatalogue(workspace: string): Promise<HubProjectPick[]> {
	if (catalogue?.workspace !== workspace) {
		const projects = loadCatalogue(workspace).catch((e) => {
			// A cached rejection would make the failure permanent for the whole session;
			// dropping it lets the next open try again.
			if (catalogue?.projects === projects) catalogue = undefined
			throw e
		})
		catalogue = { workspace, projects }
	}
	return catalogue.projects
}

/** Warms the catalogue so the picker opens on content instead of a spinner. */
export function preloadHubProjects(workspace: string): void {
	void hubProjectCatalogue(workspace).catch(() => {})
}

async function loadCatalogue(workspace: string): Promise<HubProjectPick[]> {
	const raw = await HubPublishService.listHubProjects({ workspace })
	const rows = ((typeof raw === 'string' ? JSON.parse(raw) : raw)?.projects ??
		[]) as HubProjectListRow[]
	const hub = await hubBrowserUrl()
	return rows
		.map((row) => ({
			id: row.slug,
			slug: row.slug,
			name: row.name,
			summary: row.summary,
			description: hubProjectDescription(row),
			author: row.author,
			apps: row.apps ?? [],
			logoUrl: row.hasLogo ? `${hub}/projects/${encodeURIComponent(row.slug)}/logo` : undefined,
			iconApps: row.apps ?? [],
			stars: row.stars ?? 0
		}))
		.sort((a, b) => b.stars - a.stars || a.name.localeCompare(b.name))
}
