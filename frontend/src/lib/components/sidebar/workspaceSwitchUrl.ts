import { page } from '$app/state'
import { replaceState } from '$app/navigation'
import { goto } from '$lib/navigation'
import { get } from 'svelte/store'
import {
	AppService,
	FlowService,
	JobService,
	ScriptService,
	type UserDraftItemKind
} from '$lib/gen'
import { switchWorkspace } from '$lib/storeUtils'
import { workspaceStore } from '$lib/stores'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { workspaceAIClients } from '$lib/components/copilot/lib'

// `search` is the page's query string: a `hash` or `version` in it pins one version of the
// item, which must exist in the target workspace, not just its path.
type ItemExists = (workspace: string, param: string, search: URLSearchParams) => Promise<boolean>

// A failed lookup (404 included) counts as missing.
async function succeeds(lookup: () => Promise<unknown>): Promise<boolean> {
	try {
		await lookup()
		return true
	} catch {
		return false
	}
}

const scriptHashExists = (workspace: string, hash: string) =>
	succeeds(() => ScriptService.getScriptByHash({ workspace, hash }))

// The two app editors can't open each other's kind, so the target's app must match it.
const appExists =
	(raw: boolean): ItemExists =>
	async (workspace, path) => {
		// The listing carries `raw_app` without the app value, and omits it when false.
		const [app] = await AppService.listApps({ workspace, pathExact: path, perPage: 1 })
		return !!app && !!app.raw_app === raw
	}

const flowExists: ItemExists = async (workspace, path, search) => {
	const version = Number(search.get('version') || NaN)
	if (!Number.isFinite(version)) return FlowService.existsFlowByPath({ workspace, path })
	try {
		return (await FlowService.getFlowVersion({ workspace, version })).path === path
	} catch {
		return false
	}
}

const scriptExists: ItemExists = (workspace, path, search) => {
	const hash = search.get('hash')
	return hash
		? scriptHashExists(workspace, hash)
		: ScriptService.existsScriptByPath({ workspace, path })
}

// The script page's param is a hash, a path, or a hub path.
const scriptHashOrPathExists: ItemExists = async (workspace, hashOrPath) => {
	if (hashOrPath.startsWith('hub/')) return true
	if (await ScriptService.existsScriptByPath({ workspace, path: hashOrPath })) return true
	return scriptHashExists(workspace, hashOrPath)
}

// A one-row lookup, where getJob would download the job's args and result.
const jobExists: ItemExists = (workspace, id) =>
	succeeds(() => JobService.getRootJobId({ workspace, id }))

type ItemPage = {
	param: string
	exists: ItemExists
	// Set on editors, whose auto-save-off edits are parked under this draft kind.
	draftKind?: UserDraftItemKind
}

// Pages showing one workspace item, keyed by route id, with the check that the item named
// by the route param exists in a given workspace.
const ITEM_PAGES: Record<string, ItemPage> = {
	'/(root)/(logged)/scripts/edit/[...path]': {
		param: 'path',
		exists: scriptExists,
		draftKind: 'script'
	},
	'/(root)/(logged)/scripts/get/[...hash]': { param: 'hash', exists: scriptHashOrPathExists },
	'/(root)/(logged)/flows/edit/[...path]': { param: 'path', exists: flowExists, draftKind: 'flow' },
	'/(root)/(logged)/flows/get/[...path]': { param: 'path', exists: flowExists },
	'/(root)/(logged)/apps/edit/[...path]': {
		param: 'path',
		exists: appExists(false),
		draftKind: 'app'
	},
	'/(root)/(logged)/apps/get/[...path]': { param: 'path', exists: appExists(false) },
	'/(root)/(logged)/apps_raw/edit/[...path]': {
		param: 'path',
		exists: appExists(true),
		draftKind: 'raw_app'
	},
	'/(root)/(logged)/apps_raw/get/[...path]': { param: 'path', exists: appExists(true) },
	'/(root)/(logged)/run/[...run]': { param: 'run', exists: jobExists }
}

function currentItemPage(): { itemPage: ItemPage; param: string } | undefined {
	const itemPage = page.route.id ? ITEM_PAGES[page.route.id] : undefined
	const param = itemPage ? page.params[itemPage.param] : undefined
	return itemPage && param ? { itemPage, param } : undefined
}

async function itemPageMissingIn(workspace: string): Promise<boolean> {
	const current = currentItemPage()
	if (!current) return false
	try {
		return !(await current.itemPage.exists(workspace, current.param, page.url.searchParams))
	} catch {
		return true
	}
}

// Staying on an editor would load the target workspace's copy without a pathname change, so
// its unsaved-changes guard (which only fires on one) would never ask about parked edits.
function editorHasUnsavedEdits(): boolean {
	const current = currentItemPage()
	const workspace = get(workspaceStore)
	if (!current?.itemPage.draftKind || !workspace) return false
	const byHash = current.itemPage.draftKind === 'script' && page.url.searchParams.get('hash')
	return UserDraftDbSyncer.hasUnsavedDisabledChanges({
		workspace,
		itemKind: current.itemPage.draftKind,
		path: byHash ? '' : current.param
	})
}

let latestSwitch = 0

function switchTo(id: string) {
	workspaceAIClients.init(id)
	switchWorkspace(id)
}

// Workspace switch shared by the sidebar workspace pickers. An item page stays open when
// its item also exists in `id`, and goes home otherwise.
export async function switchWorkspaceAndPage(
	id: string,
	opts?: {
		// Decided by the caller so the rule stays with the picker that has the context
		// for it — a picker rendered outside the sidebar drives its own page and must
		// not be navigated away from.
		landOnHome?: boolean
		// Where to go when staying on the page, for a picker whose items link there.
		href?: string
	}
): Promise<void> {
	const request = ++latestSwitch
	const missing = !opts?.landOnHome && (await itemPageMissingIn(id))
	// A switch picked while this lookup (or the navigation home below) was in flight wins.
	if (request !== latestSwitch) return
	// Read after the lookup: the user can edit while it is in flight.
	const unsavedEdits = editorHasUnsavedEdits()
	if (opts?.landOnHome || unsavedEdits || missing) {
		// Leave before switching: an item page still mounted when the store changes
		// refetches its item in `id` and toasts the 404.
		// The param carries the switch through the editor's unsaved-changes prompt: that
		// cancels this navigation and, on discard, replays this URL, which the logged layout
		// applies. On cancel nothing is switched.
		const from = page.route.id
		await goto(`/?workspace=${encodeURIComponent(id)}`)
		if (request !== latestSwitch) return
		if (unsavedEdits && page.route.id === from) return
		switchTo(id)
		return
	}
	switchTo(id)
	if (opts?.href) {
		await goto(opts.href)
	} else if (page.url.searchParams.get('workspace')) {
		// A stale ?workspace= param is re-applied on reload or when exiting session mode
		// restores the route, silently switching the workspace back.
		const url = new URL(window.location.href)
		url.searchParams.set('workspace', id)
		replaceState(url, page.state)
	}
}
