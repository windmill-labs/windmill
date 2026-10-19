import { onboardingProfile, type StarterPrompt } from '$lib/onboardingProfile'
import { userStore } from '$lib/stores'
import { fromStore, get } from 'svelte/store'
import { resource } from 'runed'
import {
	JobService,
	ScriptService,
	WorkerService,
	WorkspaceService,
	type RunnableItem
} from '$lib/gen'
import {
	composerSuggestions,
	type ComposerItem,
	type ComposerRole,
	type ComposerSignals,
	type ComposerSuggestion
} from './composerSuggestions'

export type { ComposerSuggestion }

/** From this many items on, Home's composer rests collapsed: below it the workspace is new
 * enough that the composer is the page's main way in. */
const FEW_ITEMS = 10

/** The suggestions for the reader's role and the workspace the session acts on, and whether
 * that workspace is still small. An invite that wrote prompts for this person replaces the
 * suggestions outright: a prompt written for someone's own stack next to "Ban Discord users"
 * reads as the generic one. */
export class StarterPrompts {
	#invite = $state<StarterPrompt[]>()
	#user = fromStore(userStore)
	#workspace: () => string | undefined
	#items: { readonly current: ItemsSignal | undefined }
	#others: { readonly current: ComposerSignals | undefined }

	/** `workspace`: where the session acts. */
	constructor(workspace: () => string | undefined) {
		this.#workspace = workspace
		void onboardingProfile().then((p) => {
			if (p?.starter_prompts?.length) this.#invite = p.starter_prompts
		})
		this.#items = resource(workspace, async (ws) => (ws ? itemsSignal(ws) : undefined))
		this.#others = resource(
			() => {
				const ws = workspace()
				const role = this.#role
				return ws && role ? { ws, role } : undefined
			},
			async (key) => (key ? otherSignals(key.ws, key.role) : undefined)
		)
	}

	/** Undefined until the user is known. The user is read for Home's workspace, so a session
	 * acting on another one (a fork) keeps only what is instance-wide. */
	get #role(): ComposerRole | undefined {
		const user = this.#user.current
		if (!user) return undefined
		const own = user.workspace_id === this.#workspace()
		return {
			operator: own && user.operator,
			workspaceAdmin: own && user.is_admin,
			instanceAdmin: user.is_super_admin
		}
	}

	/** Signals already read this page load stand in while they are asked again, so a remount
	 * does not wait on them. */
	get #itemsSignal(): ItemsSignal | undefined {
		const ws = this.#workspace()
		return this.#items.current ?? (ws ? settledItems.get(ws) : undefined)
	}

	get #otherSignals(): ComposerSignals | undefined {
		const ws = this.#workspace()
		return this.#others.current ?? (ws ? settledOthers.get(ws) : undefined)
	}

	/** Shown once the item list and every other signal have answered, so pills do not
	 * reshuffle as each request lands; an empty workspace needs only the list. */
	readonly suggestions: ComposerSuggestion[] = $derived.by(() => {
		if (this.#invite) return this.#invite.map((p) => ({ ...p, mode: 'build' as const }))
		const items = this.#itemsSignal
		const role = this.#role
		if (!items || !role) return []
		const others = this.#otherSignals
		if (!others && items.itemCount !== 0) return []
		return composerSuggestions({ ...items, ...others }, role)
	})

	/** Whether the workspace holds fewer than `FEW_ITEMS`; undefined until known. */
	get fewItems(): boolean | undefined {
		const count = this.#itemsSignal?.itemCount
		return count === undefined ? undefined : count < FEW_ITEMS
	}
}

type ItemsSignal = Pick<ComposerSignals, 'itemCount' | 'myDraft' | 'failedDeploy'>

const settledItems = new Map<string, ItemsSignal>()
const settledOthers = new Map<string, ComposerSignals>()
const requests = new Map<string, { at: number; request: Promise<unknown> }>()
/** How long an answer is reused before a mount asks again: long enough to cover the prefetch
 * that preceded the mount, short enough that returning to Home sees what changed. */
const REUSE_MS = 30_000

/** One request per workspace and signal, shared by every caller; a failure is not kept. */
function dedupe<T>(key: string, send: () => Promise<T>): Promise<T> {
	const kept = requests.get(key)
	if (kept && Date.now() - kept.at < REUSE_MS) return kept.request as Promise<T>
	const request = send()
	requests.set(key, { at: Date.now(), request })
	request.catch(() => requests.delete(key))
	return request
}

/** Starts the signals for a page about to mount the starter; the starter then takes these
 * requests rather than sending its own. The role-gated ones go only once the user is
 * known for this workspace. */
export function prefetchComposerSignals(workspace: string): void {
	void itemsSignal(workspace)
	const user = get(userStore)
	void otherSignals(workspace, {
		operator: false,
		workspaceAdmin: user?.workspace_id === workspace && user.is_admin,
		instanceAdmin: user?.is_super_admin ?? false
	})
}

/** Every signal `role` may read for `workspace`, as the starter reads them. */
export async function readComposerSignals(
	workspace: string,
	role: ComposerRole
): Promise<ComposerSignals> {
	const [items, others] = await Promise.all([itemsSignal(workspace), otherSignals(workspace, role)])
	return { ...items, ...others }
}

async function itemsSignal(workspace: string): Promise<ItemsSignal> {
	try {
		const signal = await dedupe(`${workspace}:items`, () => fetchItems(workspace))
		settledItems.set(workspace, signal)
		return signal
	} catch (e) {
		// Unknown reads as a full workspace: a collapsed composer is the quieter mistake.
		console.error(`Could not list the items of workspace ${workspace}:`, e)
		return { itemCount: FEW_ITEMS }
	}
}

async function fetchItems(workspace: string): Promise<ItemsSignal> {
	// What Home's list counts: library scripts and the user's own drafts included.
	const res = await ScriptService.listRunnables({
		workspace,
		includeWithoutMain: true,
		includeDraftOnly: true,
		orderBy: 'updated',
		orderDesc: true,
		perPage: FEW_ITEMS
	})
	const items = res.items ?? []
	const username = get(userStore)?.username
	const draft = items.find(
		(i) =>
			i.is_draft ||
			i.draft_only ||
			(username && i.draft_users?.some((u) => u['username'] === username))
	)
	const deploy = items.find((i) => i.has_deploy_errors)
	return {
		itemCount: items.length,
		myDraft: draft && composerItem(draft, draft.draft_path),
		failedDeploy: deploy && composerItem(deploy)
	}
}

function composerItem(item: RunnableItem, path = item.path): ComposerItem {
	return {
		kind: item.type,
		path,
		name: item.summary || path.split('/').pop() || path
	}
}

/** Every signal the role may ask for besides the item list, each absent if it fails. */
async function otherSignals(workspace: string, role: ComposerRole): Promise<ComposerSignals> {
	const ask = <T>(name: string, send: () => Promise<T>) =>
		dedupe(`${workspace}:${name}`, send).catch((e) => {
			console.error(`Could not read ${name} for the composer's suggestions:`, e)
			return undefined
		})
	const [failedRuns, queue, settings, workers] = await Promise.all([
		ask('failedRuns', () =>
			JobService.countCompletedJobs({ workspace, success: false, completedAfterSAgo: 86400 })
		),
		ask('queue', () => JobService.getQueueCount({ workspace })),
		role.workspaceAdmin
			? ask('settings', () => WorkspaceService.getSettings({ workspace }))
			: undefined,
		role.instanceAdmin
			? ask('workers', () => WorkerService.listWorkers({ pingSince: 300 }))
			: undefined
	])
	const signals: ComposerSignals = {
		failedRuns,
		queue: queue && { waiting: queue.database_length, suspended: queue.suspended ?? 0 },
		hasErrorHandler: settings && !!settings.error_handler,
		liveWorkers: workers?.length
	}
	settledOthers.set(workspace, signals)
	return signals
}

/** What turns an empty chat into the block that starts a session. */
export type ChatStarter = { suggestions: ComposerSuggestion[] } & (
	| /** An empty session centres the block in the pane under a greeting. */
	{ where: 'session' } /** Home shows it alone, collapsed to one line at rest unless `fewItems`. */
	| { where: 'home'; fewItems: boolean }
)
