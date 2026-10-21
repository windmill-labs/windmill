import type { Entry, Kind, Preset } from './model'

const MIN = 60_000
const HOUR = 60 * MIN
const DAY = 24 * HOUR

type Seed = { kind: Kind; path: string; summary: string; ago?: number }

const FAVORITES: Seed[] = [
	{
		kind: 'app',
		path: 'f/crm/dashboard',
		summary: 'CRM Dashboard — contacts table, deal pipeline and activity feed'
	},
	{
		kind: 'flow',
		path: 'f/crm/weekly_report',
		summary: 'Weekly CRM report: fetch contacts + deals, summarize, email to sales'
	},
	{
		kind: 'script',
		path: 'f/crm/search_contacts',
		summary: 'Search and list CRM contacts with optional filters'
	},
	{ kind: 'flow', path: 'f/billing/invoice_reminders', summary: 'Invoice reminders' },
	{ kind: 'app', path: 'u/admin/ops_console', summary: 'Ops console' },
	{ kind: 'agent', path: 'f/support/triage', summary: 'Support triage agent' },
	{
		kind: 'script',
		path: 'f/data/sync_hubspot',
		summary: 'Sync HubSpot companies into the warehouse'
	},
	{ kind: 'raw_app', path: 'f/marketing/campaign_builder', summary: 'Campaign builder' },
	{
		kind: 'flow',
		path: 'f/onboarding/new_customer',
		summary:
			'New customer onboarding: create the workspace, invite users, send the welcome email and book the kickoff call'
	},
	{ kind: 'script', path: 'f/finance/fx_rates', summary: 'Fetch daily FX rates' },
	{ kind: 'app', path: 'f/hr/pto_tracker', summary: 'PTO tracker' },
	{ kind: 'script', path: 'f/ops/restart_workers', summary: '' }
]

const RECENT: Seed[] = [
	{
		kind: 'flow',
		path: 'f/test/fake_daily_weather',
		summary: 'Generates a fake daily news article based on the weather in a given city',
		ago: 4 * MIN
	},
	{
		kind: 'app',
		path: 'u/admin/pencil_dojo',
		summary: 'Pencil Dojo — student practice dashboard',
		ago: 35 * MIN
	},
	{ kind: 'script', path: 'u/admin/amazing', summary: 'Amazing', ago: 2 * HOUR },
	{
		kind: 'raw_app',
		path: 'u/admin/doodle_jump',
		summary: 'Doodle Jump - endless vertical platformer game',
		ago: 5 * HOUR
	},
	{
		kind: 'script',
		path: 'u/admin/hello_world',
		summary: 'A simple hello world script',
		ago: 26 * HOUR
	},
	{ kind: 'flow', path: 'f/test/weather_report', summary: '', ago: 3 * DAY },
	{
		kind: 'script',
		path: 'f/etl/postgres_to_bigquery',
		summary: "Copy yesterday's orders from Postgres to BigQuery",
		ago: 6 * DAY
	}
]

const LONG_FAVORITES: Seed[] = [
	{
		kind: 'flow',
		path: 'f/finance/month_end_close',
		summary:
			'Month-end close: reconcile Stripe payouts against the ledger, flag mismatches over 50 EUR and open a ticket for each one in Linear'
	},
	{
		kind: 'app',
		path: 'f/support/escalations_board',
		summary:
			'Escalations board showing every open P1 and P2 ticket with its owner, SLA countdown and the last customer reply'
	}
]

const LONG_RECENT: Seed[] = [
	{
		kind: 'script',
		path: 'f/data/dedupe_contacts',
		summary:
			'Deduplicate CRM contacts by normalized email and phone number, keeping the most recently updated record and merging the rest',
		ago: 7 * MIN
	},
	{
		kind: 'flow',
		path: 'f/marketing/newsletter',
		summary:
			'Build the weekly newsletter from the top five blog posts, render it with the brand template and schedule it in Customer.io for Tuesday 9am',
		ago: 50 * MIN
	},
	{
		kind: 'app',
		path: 'u/admin/inventory',
		summary:
			'Inventory dashboard for the three warehouses with stock levels, reorder alerts and supplier lead times',
		ago: 3 * HOUR
	},
	{
		kind: 'script',
		path: 'f/ops/rotate_keys',
		summary:
			'Rotate the API keys of every third-party integration older than ninety days and notify their owners',
		ago: 20 * HOUR
	},
	{
		kind: 'raw_app',
		path: 'f/sales/quote_builder',
		summary:
			'Quote builder with a discount approval workflow, PDF export and automatic sync of signed quotes back to HubSpot',
		ago: 4 * DAY
	}
]

const PATH_FAVORITES: Seed[] = [
	{ kind: 'script', path: 'f/data_platform/ingestion/salesforce_accounts_daily_sync', summary: '' }
]

const PATH_RECENT: Seed[] = [
	{
		kind: 'flow',
		path: 'f/data_platform/ingestion/stripe_events_backfill',
		summary: '',
		ago: 9 * MIN
	},
	{ kind: 'script', path: 'u/admin/scratch_test_2', summary: '', ago: HOUR },
	{
		kind: 'app',
		path: 'f/internal_tools/customer_success/renewals_dashboard',
		summary: '',
		ago: 6 * HOUR
	},
	{ kind: 'script', path: 'f/ops/cleanup_old_runs', summary: '', ago: 2 * DAY },
	{
		kind: 'flow',
		path: 'f/billing/dunning_v2_experimental_retry_schedule',
		summary: '',
		ago: 5 * DAY
	}
]

const FIRST_RECENT: Seed[] = [
	{ kind: 'script', path: 'u/admin/hello_world', summary: 'Hello world', ago: 3 * MIN },
	{ kind: 'flow', path: 'u/admin/my_first_flow', summary: 'My first flow', ago: 25 * MIN }
]

// Rows the tree holds besides the shortcuts: a workspace's items are more than what one person
// starred or edited.
const OTHER_ITEMS: Seed[] = [
	{ kind: 'script', path: 'f/crm/import_csv', summary: 'Import contacts from a CSV file' },
	{ kind: 'flow', path: 'f/crm/lead_scoring', summary: 'Lead scoring' },
	{ kind: 'app', path: 'f/test/drawing_app', summary: 'Drawing app' },
	{ kind: 'raw_app', path: 'f/test/honest_app', summary: 'Dashboard' },
	{ kind: 'script', path: 'f/billing/stripe_webhook', summary: 'Handle Stripe webhooks' },
	{ kind: 'script', path: 'u/admin/test_script', summary: 'Draft-renamed summary v2' },
	{ kind: 'flow', path: 'u/admin/to_be_deleted', summary: 'to be deleted' },
	{ kind: 'script', path: 'u/guilhem/notes', summary: 'Notes' },
	{ kind: 'flow', path: 'f/e2e/flow_test', summary: 'prio test' }
]

export type TreeItem = { kind: Kind; path: string; summary: string; badges: string[] }

export type Owner = {
	icon: 'user' | 'users' | 'folder'
	name: string
	detail: string
	items: TreeItem[]
}

export type Workspace = {
	name: string
	entries: Entry[]
	owners: Owner[]
	drafts: number
}

function entries(favorites: Seed[], recent: Seed[], now: number): Entry[] {
	return [
		...favorites.map((s) => ({ kind: s.kind, path: s.path, summary: s.summary, favorite: true })),
		...recent.map((s) => ({
			kind: s.kind,
			path: s.path,
			summary: s.summary,
			favorite: false,
			at: now - (s.ago ?? 0)
		}))
	]
}

function plural(n: number, word: string): string {
	return `${n} ${word}${n === 1 ? '' : 's'}`
}

// The tree as Home groups it: your own space, then other users folded into one row, then
// folders, each listing its items.
function owners(seeds: Seed[]): Owner[] {
	const byOwner = new Map<string, TreeItem[]>()
	for (const s of seeds) {
		const owner = s.path.split('/').slice(0, 2).join('/')
		const list = byOwner.get(owner) ?? []
		if (list.some((i) => i.path === s.path)) continue
		const badges = s.kind === 'raw_app' ? ['Raw'] : []
		if (s.path.endsWith('honest_app') || s.path.endsWith('weekly_report')) badges.push('Draft')
		list.push({ kind: s.kind, path: s.path, summary: s.summary, badges })
		byOwner.set(owner, list)
	}
	const mine = byOwner.get('u/admin') ?? []
	const others = [...byOwner.entries()].filter(([o]) => o.startsWith('u/') && o !== 'u/admin')
	const folders = [...byOwner.entries()].filter(([o]) => o.startsWith('f/')).sort()
	const result: Owner[] = []
	if (mine.length) {
		result.push({
			icon: 'user',
			name: 'u/admin',
			detail: plural(mine.length, 'item'),
			items: mine
		})
	}
	if (others.length) {
		const items = others.flatMap(([, i]) => i)
		result.push({
			icon: 'users',
			name: 'Other users',
			detail: `${plural(others.length, 'user')} · ${plural(items.length, 'item')}`,
			items
		})
	}
	for (const [name, items] of folders) {
		result.push({ icon: 'folder', name, detail: plural(items.length, 'item'), items })
	}
	return result
}

export function workspaceFor(preset: Preset, now: number): Workspace {
	switch (preset) {
		case 'new':
			return { name: 'empty', entries: [], owners: [], drafts: 0 }
		case 'first':
			return {
				name: 'my-first-workspace',
				entries: entries([], FIRST_RECENT, now),
				owners: owners(FIRST_RECENT),
				drafts: 0
			}
		case 'single':
			return {
				name: 'acme',
				entries: entries([], RECENT.slice(0, 1), now),
				owners: owners([...RECENT, ...OTHER_ITEMS]),
				drafts: 2
			}
		case 'favorites':
			return {
				name: 'acme',
				entries: entries(FAVORITES.slice(0, 4), [], now),
				owners: owners([...FAVORITES, ...OTHER_ITEMS]),
				drafts: 4
			}
		case 'busy':
			return {
				name: 'acme',
				entries: entries(FAVORITES.slice(0, 10), RECENT.slice(0, 6), now),
				owners: owners([...FAVORITES, ...RECENT, ...OTHER_ITEMS]),
				drafts: 28
			}
		case 'long':
			return {
				name: 'acme',
				entries: entries(LONG_FAVORITES, LONG_RECENT, now),
				owners: owners([...LONG_FAVORITES, ...LONG_RECENT, ...OTHER_ITEMS]),
				drafts: 6
			}
		case 'paths':
			return {
				name: 'acme',
				entries: entries(PATH_FAVORITES, PATH_RECENT, now),
				owners: owners([...PATH_FAVORITES, ...PATH_RECENT, ...OTHER_ITEMS]),
				drafts: 6
			}
		case 'typical':
		default:
			return {
				name: 'acme',
				entries: entries(FAVORITES.slice(0, 2), RECENT.slice(0, 5), now),
				owners: owners([...FAVORITES.slice(0, 4), ...RECENT, ...OTHER_ITEMS]),
				drafts: 12
			}
	}
}
