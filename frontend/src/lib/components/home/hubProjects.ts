/**
 * Hub projects offered as a starting point on an otherwise empty workspace.
 *
 * The shape mirrors what the Hub's `GET /projects/<slug>` returns with
 * `Accept: application/json` — the presentation-only summary, not the ~200KB export
 * bundle. See windmillhub#180.
 */
export interface HubProject {
	slug: string
	name: string
	summary: string
	author: string
	/** Integration keys, resolvable through `appIconComponent`. */
	apps: string[]
	hasLogo: boolean
	stars: number
	/** Item breakdown, e.g. `{ script: 5, flow: 1, app: 1 }`. */
	counts: Record<string, number>
}

export const HUB_BASE_URL = 'https://hub.windmill.dev'

export function hubProjectLogoUrl(slug: string): string {
	return `${HUB_BASE_URL}/projects/${slug}/logo`
}

export function hubProjectUrl(slug: string): string {
	return `${HUB_BASE_URL}/projects/${slug}`
}

/**
 * The Hub has no "most popular projects" endpoint yet, so this list stands in until it
 * lands. Keep the call site async: swapping in the fetch should not change the caller.
 */
const SEEDED_POPULAR: HubProject[] = [
	{
		slug: 'support-automation',
		name: 'Support automation',
		summary:
			'AI support triage with a human approval gate: classify, analyze, draft the reply, dispatch clear-cut fixes in parallel.',
		author: 'hugo989',
		apps: ['anthropic', 'github', 'sendgrid', 'slack'],
		hasLogo: true,
		stars: 0,
		counts: { script: 7, flow: 1, app: 1 }
	},
	{
		slug: 'bitly',
		name: 'Bitly',
		summary: 'Short link shortener with click analytics',
		author: 'tristan795',
		apps: [],
		hasLogo: true,
		stars: 0,
		counts: { script: 5, app: 1 }
	},
	{
		slug: 'recruit',
		name: 'Recruit',
		summary: 'Application form + AI CV screening',
		author: 'tristan795',
		apps: ['anthropic'],
		hasLogo: true,
		stars: 0,
		counts: { script: 4, flow: 1, app: 1 }
	},
	{
		slug: 'calendly',
		name: 'Calendly',
		summary: 'Booking page + availability engine',
		author: 'tristan795',
		apps: ['gcal', 'smtp'],
		hasLogo: true,
		stars: 0,
		counts: { script: 6, app: 2 }
	},
	{
		slug: 'uptimerobot',
		name: 'Uptimerobot',
		summary: 'Uptime monitoring with self-managed schedules',
		author: 'tristan795',
		apps: ['smtp'],
		hasLogo: true,
		stars: 0,
		counts: { script: 5, app: 1 }
	},
	{
		slug: 'odoo',
		name: 'Odoo',
		summary: 'Manages Odoo records interactively.',
		author: 'drpsyko101653',
		apps: ['misc'],
		hasLogo: true,
		stars: 2,
		counts: { script: 8 }
	}
]

export async function fetchPopularHubProjects(limit = 6): Promise<HubProject[]> {
	return SEEDED_POPULAR.slice(0, limit)
}

/** "7 scripts · 1 flow · 1 app", in a fixed order so cards line up. */
export function formatItemCounts(counts: Record<string, number>): string {
	const labels: [string, string, string][] = [
		['script', 'script', 'scripts'],
		['flow', 'flow', 'flows'],
		['app', 'app', 'apps']
	]
	return labels
		.map(([key, one, many]) => {
			const n = counts[key] ?? 0
			return n > 0 ? `${n} ${n === 1 ? one : many}` : undefined
		})
		.filter(Boolean)
		.join(' · ')
}
