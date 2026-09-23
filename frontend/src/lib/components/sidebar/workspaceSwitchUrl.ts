import { page } from '$app/state'
import { replaceState } from '$app/navigation'
import { goto } from '$lib/navigation'
import { AppService, FlowService, JobService, ScriptService } from '$lib/gen'
import { switchWorkspace } from '$lib/storeUtils'

type ItemExists = (workspace: string, param: string) => Promise<boolean>

const appExists: ItemExists = (workspace, path) => AppService.existsApp({ workspace, path })
const flowExists: ItemExists = (workspace, path) =>
	FlowService.existsFlowByPath({ workspace, path })
const scriptExists: ItemExists = (workspace, path) =>
	ScriptService.existsScriptByPath({ workspace, path })

// The script page's param is a hash, a path, or a hub path.
const scriptHashOrPathExists: ItemExists = async (workspace, hashOrPath) => {
	if (hashOrPath.startsWith('hub/')) return true
	if (await scriptExists(workspace, hashOrPath)) return true
	try {
		await ScriptService.getScriptByHash({ workspace, hash: hashOrPath })
		return true
	} catch {
		return false
	}
}

// A one-row lookup, where getJob would download the job's args and result.
const jobExists: ItemExists = async (workspace, id) => {
	try {
		await JobService.getRootJobId({ workspace, id })
		return true
	} catch {
		return false
	}
}

// Pages showing one workspace item, keyed by route id, with the check that the item named
// by the route param exists in a given workspace.
const ITEM_PAGES: Record<string, { param: string; exists: ItemExists }> = {
	'/(root)/(logged)/scripts/edit/[...path]': { param: 'path', exists: scriptExists },
	'/(root)/(logged)/scripts/get/[...hash]': { param: 'hash', exists: scriptHashOrPathExists },
	'/(root)/(logged)/flows/edit/[...path]': { param: 'path', exists: flowExists },
	'/(root)/(logged)/flows/get/[...path]': { param: 'path', exists: flowExists },
	'/(root)/(logged)/apps/edit/[...path]': { param: 'path', exists: appExists },
	'/(root)/(logged)/apps/get/[...path]': { param: 'path', exists: appExists },
	'/(root)/(logged)/apps_raw/edit/[...path]': { param: 'path', exists: appExists },
	'/(root)/(logged)/apps_raw/get/[...path]': { param: 'path', exists: appExists },
	'/(root)/(logged)/run/[...run]': { param: 'run', exists: jobExists }
}

async function itemPageMissingIn(workspace: string): Promise<boolean> {
	const itemPage = page.route.id ? ITEM_PAGES[page.route.id] : undefined
	const param = itemPage ? page.params[itemPage.param] : undefined
	if (!itemPage || !param) return false
	try {
		return !(await itemPage.exists(workspace, param))
	} catch {
		return true
	}
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
	if (opts?.landOnHome || (await itemPageMissingIn(id))) {
		// Leave before switching: an item page still mounted when the store changes
		// refetches its item in `id` and toasts the 404.
		await goto('/')
		switchWorkspace(id)
		return
	}
	switchWorkspace(id)
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
