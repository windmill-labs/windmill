import { onboardingProfile, type StarterPrompt } from '$lib/onboardingProfile'
import { roleCapabilities, userStore } from '$lib/stores'
import { canEditItem } from '$lib/editRights'
import { scopedKey } from '$lib/userScopedStorage'
import { fromStore, get } from 'svelte/store'
import { resource } from 'runed'
import {
	ResourceService,
	ScriptService,
	WorkspaceService,
	type ListableResource,
	type RunnableItem
} from '$lib/gen'
import {
	CHANNEL_TYPES,
	composerSuggestions,
	DATABASE_TYPES,
	FEW_ITEMS,
	type ComposerItem,
	type ComposerResource,
	type ComposerRole,
	type ComposerSignals,
	type ComposerSuggestion
} from './composerSuggestions'

export type { ComposerSuggestion }

/** The newest items read: enough to find a shared flow beside the reader's own work. */
const ITEMS_READ = 25
/** The connections read. A full page may have cut the channel off, so only a page shorter
 * than this proves the workspace has none. */
const RESOURCES_READ = 20

/** The suggestions for the reader's role and the workspace the session acts on, and whether
 * that workspace is still small. An invite that wrote prompts for this person replaces the
 * suggestions outright: a prompt written for someone's own stack next to "Approve refunds in
 * Slack" reads as the generic one. */
export class StarterPrompts {
	#invite = $state<StarterPrompt[]>()
	#user = fromStore(userStore)
	#capabilities = fromStore(roleCapabilities)
	#workspace: () => string | undefined
	#signals: { readonly current: Tagged<ComposerSignals> | undefined }

	/** `workspace`: where the session acts. */
	constructor(workspace: () => string | undefined) {
		this.#workspace = workspace
		void onboardingProfile().then((p) => {
			if (p?.starter_prompts?.length) this.#invite = p.starter_prompts
		})
		// runed keeps whichever fetch settles last, so after a workspace or account switch an
		// earlier fetch can land on top of the current one: each answer carries what it is for.
		this.#signals = resource(
			() => this.#tag,
			async (tag) => {
				const ws = this.#workspace()
				return ws ? { tag, value: await readComposerSignals(ws) } : undefined
			}
		)
	}

	get #tag(): string {
		return `${this.#workspace()}\0${this.#user.current?.email}`
	}

	/** Undefined until the user is known. The user is read for Home's workspace, so a session
	 * acting on another one (a fork) keeps only what is instance-wide. */
	get #role(): ComposerRole | undefined {
		const user = this.#user.current
		if (!user) return undefined
		const own = user.workspace_id === this.#workspace()
		return {
			operator: own && !this.#capabilities.current.code,
			workspaceAdmin: own && user.is_admin,
			instanceAdmin: user.is_super_admin
		}
	}

	/** Signals already read this page load stand in while they are asked again, so a remount
	 * does not wait on them. */
	get #currentSignals(): ComposerSignals | undefined {
		const current = this.#signals.current
		if (current?.tag === this.#tag) return current.value
		const ws = this.#workspace()
		const key = ws && signalsKey(ws)
		return key ? settled.get(key) : undefined
	}

	readonly suggestions: ComposerSuggestion[] = $derived.by(() => {
		if (this.#invite) return this.#invite.map((p) => ({ ...p, mode: 'build' as const }))
		const signals = this.#currentSignals
		const role = this.#role
		return signals && role ? composerSuggestions(signals, role) : []
	})

	/** Whether the workspace holds fewer than `FEW_ITEMS`, below which Home's composer rests
	 * open as the page's main way in; undefined until known. */
	get fewItems(): boolean | undefined {
		const count = this.#currentSignals?.itemCount
		return count === undefined ? undefined : count < FEW_ITEMS
	}
}

type Tagged<T> = { tag: string; value: T }

/** Per account: the signals name the reader's private draft. A key is taken when a request
 * starts, so one that settles after an account switch cannot answer for the next account; with
 * no account known yet, nothing is cached. */
const signalsKey = (workspace: string) => scopedKey(`composerSignals:${workspace}`)

const settled = new Map<string, ComposerSignals>()
const requests = new Map<string, { at: number; request: Promise<ComposerSignals> }>()
/** How long an answer is reused before a mount asks again: long enough to cover the prefetch
 * that preceded the mount, short enough that returning to Home sees what changed. */
const REUSE_MS = 30_000

/** Starts the reads for a page about to mount the starter; the starter then takes these
 * requests rather than sending its own. */
export function prefetchComposerSignals(workspace: string): void {
	void readComposerSignals(workspace)
}

/** What the starter reads for `workspace`: one set of requests, shared by every caller and
 * reused for `REUSE_MS`; a failure is not kept. */
export async function readComposerSignals(workspace: string): Promise<ComposerSignals> {
	const key = signalsKey(workspace)
	try {
		let request: Promise<ComposerSignals>
		const kept = key && requests.get(key)
		if (kept && Date.now() - kept.at < REUSE_MS) request = kept.request
		else {
			request = fetchSignals(workspace)
			if (key) {
				requests.set(key, { at: Date.now(), request })
				request.catch(() => {
					if (requests.get(key)?.request === request) requests.delete(key)
				})
			}
		}
		const signals = await request
		if (!key) return signals
		// A request that outlived the reuse window can settle after the one that replaced it.
		if (requests.get(key)?.request === request) settled.set(key, signals)
		return settled.get(key) ?? signals
	} catch (e) {
		// Unknown reads as a full workspace: a collapsed composer is the quieter mistake.
		console.error(`Could not list the items of workspace ${workspace}:`, e)
		return { itemCount: FEW_ITEMS }
	}
}

async function fetchSignals(workspace: string): Promise<ComposerSignals> {
	// Read before the requests: a workspace switch while they are pending moves the user store
	// to the next workspace's membership.
	const member = get(userStore)
	const user = member?.workspace_id === workspace ? member : undefined
	const capabilities = get(roleCapabilities)
	const [res, resources, dataTables] = await Promise.all([
		// What Home's list counts: library scripts and the user's own drafts included.
		ScriptService.listRunnables({
			workspace,
			includeWithoutMain: true,
			includeDraftOnly: true,
			orderBy: 'updated',
			orderDesc: true,
			perPage: ITEMS_READ
		}),
		readConnections(workspace),
		readDataTables(workspace)
	])
	const items = res.items ?? []
	const deployed = items.filter((i) => !i.draft_only)
	const username = user?.username
	const draft = items.find(
		(i) =>
			i.is_draft ||
			i.draft_only ||
			(username && i.draft_users?.some((u) => u['username'] === username))
	)
	const editable = (i: RunnableItem) =>
		canEditItem(
			i.type === 'app' && i.raw_app ? 'raw_app' : i.type,
			i.path,
			i.extra_perms,
			user,
			capabilities
		)
	const shared = deployed.find(
		(i) => i.type === 'flow' && !(username && i.path.startsWith(`u/${username}/`))
	)
	// Only plain scripts run on their own: a library has no main function, a pipeline script
	// runs as its pipeline, and test and preprocessor scripts serve another runnable.
	const script = deployed.find(
		(i) =>
			i.type === 'script' && (!i.auto_kind || i.auto_kind === 'script' || i.auto_kind === 'wac')
	)
	const starred = deployed.find((i) => i.starred && (i.type === 'script' || i.type === 'flow'))
	const pick = (item: RunnableItem | undefined) => item && composerItem(item)
	const complete = resources !== undefined && resources.length < RESOURCES_READ
	return {
		itemCount: items.length,
		myDraft: draft && composerItem(draft, draft.draft_path),
		sharedFlow: pick(shared),
		script: pick(script),
		app: pick(deployed.find((i) => i.type === 'app' && i.raw_app && editable(i))),
		agent: pick(deployed.find((i) => i.type === 'agent')),
		folderItem: pick(deployed.find((i) => i.path.startsWith('f/'))),
		favorite: pick(starred),
		database:
			resources && newest(resources.filter((r) => Object.hasOwn(DATABASE_TYPES, r.resource_type))),
		channel: resources && (preferredChannel(resources) ?? (complete ? null : undefined)),
		dataTables
	}
}

/** The workspace's databases and channels; unknown when the list fails, which fires none of
 * the pills that read them. */
async function readConnections(workspace: string): Promise<ListableResource[] | undefined> {
	try {
		return await ResourceService.listResource({
			workspace,
			resourceType: [...Object.keys(DATABASE_TYPES), ...Object.keys(CHANNEL_TYPES)].join(','),
			perPage: RESOURCES_READ
		})
	} catch (e) {
		console.error(`Could not list the connections of workspace ${workspace}:`, e)
		return undefined
	}
}

/** The workspace's data table names, `main` (the one scripts use by default) first; unknown
 * when the list fails. */
async function readDataTables(workspace: string): Promise<string[] | undefined> {
	try {
		const names = (await WorkspaceService.listDataTables({ workspace })).map((d) => d.name)
		return names.includes('main') ? ['main', ...names.filter((n) => n !== 'main')] : names
	} catch (e) {
		console.error(`Could not list the data tables of workspace ${workspace}:`, e)
		return undefined
	}
}

function preferredChannel(resources: ListableResource[]): ComposerResource | undefined {
	for (const type of Object.keys(CHANNEL_TYPES)) {
		const channel = newest(resources.filter((r) => r.resource_type === type))
		if (channel) return channel
	}
	return undefined
}

function newest(resources: ListableResource[]): ComposerResource | undefined {
	const r = resources.reduce<ListableResource | undefined>(
		(a, b) => (!a || (b.edited_at ?? '') > (a.edited_at ?? '') ? b : a),
		undefined
	)
	return r?.path && r.resource_type
		? { type: r.resource_type, path: r.path, name: r.path.split('/').pop() || r.path }
		: undefined
}

function composerItem(item: RunnableItem, path = item.path): ComposerItem {
	return {
		kind: item.type,
		path,
		name: item.summary || path.split('/').pop() || path
	}
}

/** What turns an empty chat into the block that starts a session. */
export type ChatStarter = { suggestions: ComposerSuggestion[] } & (
	| /** An empty session centres the block in the pane under a greeting. */
	{ where: 'session' } /** Home shows it alone, collapsed to one line at rest unless `fewItems`. */
	| { where: 'home'; fewItems: boolean }
)
