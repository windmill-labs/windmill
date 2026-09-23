import type { Flow, NewSchedule, NewScript } from '$lib/gen/types.gen'
import { AppService, DraftService, FlowService, ScriptService } from '$lib/gen'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import { UserDraft, type UserDraftEntry, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
import {
	getWorkspaceItemKey,
	type AppDraftValue,
	type ResourceDraftState,
	type TriggerKind,
	type TriggerRequestBody,
	type VariableDraftState,
	type WorkspaceItem,
	type WorkspaceItemType
} from './workspaceItems'

const TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND = {
	http: 'trigger_http',
	websocket: 'trigger_websocket',
	kafka: 'trigger_kafka',
	nats: 'trigger_nats',
	postgres: 'trigger_postgres',
	mqtt: 'trigger_mqtt',
	amqp: 'trigger_amqp',
	sqs: 'trigger_sqs',
	gcp: 'trigger_gcp',
	azure: 'trigger_azure',
	email: 'trigger_email'
} as const satisfies Record<TriggerKind, UserDraftItemKind>

const TRIGGER_KIND_BY_DRAFT_KIND = Object.fromEntries(
	Object.entries(TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND).map(([triggerKind, draftKind]) => [
		draftKind,
		triggerKind
	])
) as Partial<Record<UserDraftItemKind, TriggerKind>>

const GLOBAL_DRAFT_KINDS = [
	'script',
	'flow',
	'raw_app',
	'trigger_schedule',
	'trigger_http',
	'trigger_websocket',
	'trigger_kafka',
	'trigger_nats',
	'trigger_postgres',
	'trigger_mqtt',
	'trigger_amqp',
	'trigger_sqs',
	'trigger_gcp',
	'trigger_azure',
	'trigger_email',
	'resource',
	'variable'
] as const satisfies UserDraftItemKind[]

function clone<T>(value: T): T {
	return structuredClone(value) as T
}

function normalizeAppDraftValue(value: AppDraftValue): AppDraftValue {
	return {
		summary: value.summary,
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: value.data ?? { ...DEFAULT_RAW_APP_DATA },
		policy: value.policy === undefined ? undefined : clone(value.policy),
		custom_path: value.custom_path,
		// Carry the fork-base version through the whitelist — it is dropped on every
		// save otherwise, which would defeat the stale-draft check.
		parent_version: value.parent_version,
		// Same for the friendly path of a draft-only app: dropping it here would
		// rename the app back to its `draft_<uuid>` storage key on every chat edit.
		draft_path: value.draft_path
	}
}

function getItemSummary(value: unknown): string | undefined {
	return ((value as { summary?: string | null } | undefined)?.summary ?? undefined) || undefined
}

export function itemKindFor(
	type: WorkspaceItemType,
	triggerKind?: TriggerKind
): UserDraftItemKind | undefined {
	switch (type) {
		case 'script':
		case 'flow':
		case 'resource':
		case 'variable':
			return type
		case 'app':
			return 'raw_app'
		case 'schedule':
			return 'trigger_schedule'
		case 'trigger':
			return triggerKind ? TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[triggerKind] : undefined
	}
}

export function triggerKindToUserDraftKind(kind: TriggerKind): UserDraftItemKind {
	return TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[kind]
}

/** Inverse of `itemKindFor`: the chat-facing type (+ trigger kind) for a draft
 * kind. Classic and raw app drafts both surface as the chat's `app` type —
 * mirroring the read path, which pairs the two kinds. `undefined` for kinds
 * the chat cannot address (webhook / poll / cli trigger drafts, data
 * pipelines). */
export function itemTypeForKind(
	kind: UserDraftItemKind
): { type: WorkspaceItemType; triggerKind?: TriggerKind } | undefined {
	switch (kind) {
		case 'script':
		case 'flow':
		case 'resource':
		case 'variable':
			return { type: kind }
		case 'app':
		case 'raw_app':
			return { type: 'app' }
		case 'trigger_schedule':
			return { type: 'schedule' }
		default: {
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[kind]
			return triggerKind ? { type: 'trigger', triggerKind } : undefined
		}
	}
}

function scriptDraftToWorkspaceItem(path: string, draft: NewScript): WorkspaceItem {
	return {
		type: 'script',
		path,
		// A script's chosen name is the value's own `path` — the editor binds the Path
		// widget to it — so a rename or a `draft_<uuid>` storage key shows up there, and
		// the session editor may also park one in `draft_path`.
		draftPath: chosenDraftName('script', draft) ?? (draft as { draft_path?: string }).draft_path,
		summary: draft.summary,
		language: draft.language,
		value: draft.content,
		schema: draft.schema,
		parentHash: draft.parent_hash,
		isDraft: true
	}
}

function flowDraftToWorkspaceItem(path: string, draft: Flow): WorkspaceItem {
	return {
		type: 'flow',
		path,
		draftPath: (draft as Flow & { draft_path?: string }).draft_path,
		summary: draft.summary,
		// The persisted flow draft carries `version_id` (the deployed head it was
		// forked from, pinned at fork by writeDraft/the editor) — the flow analog
		// of a script's parent_hash.
		parentVersionId: draft.version_id,
		value: {
			value: draft.value,
			schema: draft.schema ?? null,
			groups: draft.value.groups ?? null,
			description: draft.description ?? null
		},
		isDraft: true
	}
}

function appDraftToWorkspaceItem(path: string, draft: AppDraftValue): WorkspaceItem {
	const value = normalizeAppDraftValue(draft)
	return {
		type: 'app',
		path,
		draftPath: value.draft_path,
		summary: value.summary,
		parentVersionId: value.parent_version,
		value,
		// The chat only ever addresses the `raw_app` draft kind (see itemKindFor),
		// so every app draft it can see is a code app.
		rawApp: true,
		isDraft: true
	}
}

function scheduleDraftToWorkspaceItem(path: string, draft: NewSchedule): WorkspaceItem {
	return {
		type: 'schedule',
		path,
		summary: draft.summary ?? undefined,
		value: clone(draft),
		isDraft: true
	}
}

function triggerDraftToWorkspaceItem(
	kind: TriggerKind,
	path: string,
	draft: TriggerRequestBody
): WorkspaceItem {
	return {
		type: 'trigger',
		triggerKind: kind,
		path,
		summary: getItemSummary(draft),
		value: clone(draft),
		isDraft: true
	}
}

function resourceDraftToWorkspaceItem(path: string, draft: ResourceDraftState): WorkspaceItem {
	return {
		type: 'resource',
		path,
		summary: draft.description || undefined,
		value: {
			path,
			value: clone(draft.args),
			description: draft.description,
			resource_type: draft.resource_type ?? '',
			labels: draft.labels,
			ws_specific: draft.wsSpecific
		},
		isDraft: true
	}
}

function variableDraftToWorkspaceItem(path: string, draft: VariableDraftState): WorkspaceItem {
	return {
		type: 'variable',
		path,
		summary: draft.variable.description || undefined,
		isSecret: draft.variable.is_secret,
		value: {
			path,
			value: draft.variable.value,
			is_secret: draft.variable.is_secret,
			description: draft.variable.description,
			account: draft.account,
			is_oauth: draft.is_oauth,
			expires_at: draft.expires_at,
			labels: draft.labels,
			ws_specific: draft.wsSpecific
		},
		isDraft: true
	}
}

function userDraftEntryToWorkspaceItem(
	entry: UserDraftEntry,
	path = entry.path,
	isLiveDraft = false
): WorkspaceItem | undefined {
	let item: WorkspaceItem | undefined
	switch (entry.itemKind) {
		case 'script':
			item = scriptDraftToWorkspaceItem(path, entry.value as NewScript)
			break
		case 'flow':
			item = flowDraftToWorkspaceItem(path, entry.value as Flow)
			break
		case 'raw_app':
			item = appDraftToWorkspaceItem(path, entry.value as AppDraftValue)
			break
		case 'trigger_schedule':
			item = scheduleDraftToWorkspaceItem(path, entry.value as NewSchedule)
			break
		case 'resource':
			item = resourceDraftToWorkspaceItem(path, entry.value as ResourceDraftState)
			break
		case 'variable':
			item = variableDraftToWorkspaceItem(path, entry.value as VariableDraftState)
			break
		default: {
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[entry.itemKind]
			item = triggerKind
				? triggerDraftToWorkspaceItem(triggerKind, path, entry.value as TriggerRequestBody)
				: undefined
		}
	}
	if (!item) return undefined
	// Drop a draftPath that just repeats `path` (no extra display information),
	// keep it otherwise — including on live entries: a live editor that
	// registers its storage key as the effective path (flow/raw-app renames
	// live in the value's `draft_path`, not `path`) must not hide the staged
	// rename from lists and pickers.
	const draftPath = item.draftPath === item.path ? undefined : item.draftPath
	return isLiveDraft ? { ...item, draftPath, isLiveDraft: true } : { ...item, draftPath }
}

function liveDisplayPath(
	workspace: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): { displayPath: string; isLiveDraft: boolean } {
	const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
	if (liveDraft?.storagePath !== storagePath) {
		return { displayPath: storagePath, isLiveDraft: false }
	}
	return {
		displayPath: liveDraft.effectivePath || storagePath,
		isLiveDraft: true
	}
}

type StagedDraft = { storagePath: string; summary?: string }

// Scripts keep their chosen path in the value's own `path`; the other kinds in `draft_path`
// (the same split listDrafts applies server-side).
export function chosenDraftName(itemKind: UserDraftItemKind, value: unknown): string | undefined {
	const v = value as { path?: string; draft_path?: string } | null | undefined
	return (itemKind === 'script' ? v?.path : v?.draft_path) || undefined
}

const deployedExists: Partial<
	Record<UserDraftItemKind, (workspace: string, path: string) => Promise<boolean>>
> = {
	script: (workspace, path) => ScriptService.existsScriptByPath({ workspace, path }),
	flow: (workspace, path) => FlowService.existsFlowByPath({ workspace, path }),
	raw_app: (workspace, path) => AppService.existsApp({ workspace, path })
}

/** What a path resolved to: where the draft is stored, and its value when reaching it
 * already fetched one, so a read does not ask for the same draft twice. */
type ResolvedDraft = { storagePath: string; value?: unknown }

/**
 * Where the draft a path names is stored. A new item is stored at a generated
 * `draft_<uuid>` path until it is deployed, and a rename is staged over the old path, so
 * the name the user and the model use is often not where the draft lives.
 *
 * Resolved per call against the workspace's drafts as they are now. Nothing is cached: a
 * name that led to one draft a moment ago can name another, and a write that follows a
 * stale name creates a second draft beside the one it meant to edit.
 */
async function resolveDraft(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	/** A write must know which draft a name belongs to; a read can fall back to the path
	 * itself, which is what it addressed before names were resolved at all. */
	opts: { forWrite?: boolean } = {}
): Promise<ResolvedDraft> {
	const live = liveEditorStoragePath(workspace, itemKind, path)
	if (live !== path) return { storagePath: live }
	// A draft stored right at this path needs no name lookup — the common case, since the
	// chat is handed storage paths.
	if (UserDraft.get(itemKind, path, { workspace }) !== undefined) return { storagePath: path }
	const own = await fetchBackendDraftValue(workspace, itemKind, path)
	if (own !== undefined) return { storagePath: path, value: own }
	return { storagePath: await storagePathForChosenName(workspace, itemKind, path, opts) }
}

async function storagePathForChosenName(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string,
	opts: { forWrite?: boolean }
): Promise<string> {
	const staged: StagedDraft[] = []
	const add = (storagePath: string, summary: string | undefined) => {
		// A new script or flow editor stores under '', which no tool call can address.
		if (!storagePath || storagePath === path) return
		if (staged.some((s) => s.storagePath === storagePath)) return
		staged.push({ storagePath, summary })
	}
	let rows: Awaited<ReturnType<typeof DraftService.listDrafts>>
	try {
		rows = await DraftService.listDrafts({ workspace })
	} catch (e) {
		// Nothing is stored at this path, so it may be a name. Without the listing that
		// cannot be told, and a write that guessed the path itself would create a second
		// draft; a read of whatever is deployed there is harmless.
		if (!opts.forWrite) return path
		throw new Error(
			`Could not load this workspace's drafts, so "${path}" cannot be matched to the draft ` +
				`it may name: ${e instanceof Error ? e.message : String(e)}. Try again.`
		)
	}
	for (const row of rows) {
		if (row.kind === itemKind && row.draft_path === path) add(row.path, row.summary)
	}
	// Local cells too: a name typed a moment ago may not have reached the backend yet.
	for (const entry of UserDraft.list({ workspace, itemKinds: [itemKind] })) {
		if (chosenDraftName(itemKind, entry.value) === path) {
			add(entry.path, getItemSummary(entry.value))
		}
	}
	if (staged.length === 0) return path
	// A deployed item keeps its own path: drafts staged under that name belong to other
	// items and are reached by their storage path.
	const exists = deployedExists[itemKind]
	if (exists && (await exists(workspace, path))) return path
	if (staged.length > 1) {
		// Refused rather than guessed: nothing checks a name for uniqueness before deploy.
		const listed = staged
			.map((s) => (s.summary ? `${s.storagePath} ("${s.summary}")` : s.storagePath))
			.join(', ')
		throw new Error(
			`Several drafts are staged under "${path}": ${listed}. Pass the path of the one you mean.`
		)
	}
	return staged[0].storagePath
}

/** The open editor's staged rename alone, for callers that already hold a storage path.
 * Synchronous, so a `$derived` can call it. */
function liveEditorStoragePath(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): string {
	const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
	if (liveDraft && (path === liveDraft.storagePath || path === liveDraft.effectivePath))
		return liveDraft.storagePath
	return path
}

/** Follows the open editor's staged rename only, for callers that already hold a storage
 * path. A path that may be a draft's chosen name needs `resolveGlobalDraftStoragePath`. */
export function liveGlobalDraftStoragePath(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): string {
	const itemKind = itemKindFor(type, triggerKind)
	return itemKind ? liveEditorStoragePath(workspace, itemKind, path) : path
}

/** One resolution for a whole operation, so a tool reads its merge base, writes and
 * cleans up at the same key. Resolving twice lets a listing that fails in between send
 * the two halves to different drafts. `value` is the draft found while resolving, if any;
 * `fetched` says the backend was already asked, so no caller asks again. */
export async function resolveGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind,
	opts: { forWrite?: boolean } = {}
): Promise<{ storagePath: string; value?: unknown; fetched: boolean }> {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return { storagePath: path, fetched: false }
	const resolved = await resolveDraft(workspace, itemKind, path, opts)
	const cell = UserDraft.get(itemKind, resolved.storagePath, { workspace })
	return {
		storagePath: resolved.storagePath,
		value: cell ?? resolved.value,
		// Only the path it probed was fetched; a name resolved elsewhere was not.
		fetched: cell !== undefined || resolved.storagePath === path
	}
}

/** Where the draft a model-supplied path names is stored. */
export async function resolveGlobalDraftStoragePath(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): Promise<string> {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return path
	return (await resolveDraft(workspace, itemKind, path)).storagePath
}

function getGlobalDraftSlot(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
) {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const storagePath = liveEditorStoragePath(workspace, itemKind, path)
	const draft = UserDraft.get(itemKind, storagePath, { workspace })
	if (draft === undefined) return undefined

	const { displayPath, isLiveDraft } = liveDisplayPath(workspace, itemKind, storagePath)
	const entry = {
		workspace,
		itemKind,
		path: storagePath,
		value: draft,
		meta: {},
		persisted: false,
		live: false
	}
	const item = userDraftEntryToWorkspaceItem(entry, displayPath, isLiveDraft)
	if (!item) return undefined
	return { itemKind, storagePath, displayPath, item }
}

// Current user's persisted draft value (+ records the sync baseline so a later
// save detects external conflicts). undefined when no draft exists at that path.
// Uses `getOwnDraft` (not `getDraftForUser`): the latter rejects drawer kinds
// (schedule/trigger/resource/variable drafts are private to their owner), which
// would make those drafts write-only here — listed but never readable/deployable.
// Errors (403/500/network) MUST propagate: swallowing one would make the write
// merge fall through to the deployed item instead of the user's in-progress
// draft, silently overwriting their draft-only changes.
async function fetchBackendDraftValue(
	workspace: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): Promise<unknown | undefined> {
	const resp = await DraftService.getOwnDraft({
		workspace,
		kind: itemKind,
		path: storagePath
	})
	if (!resp) return undefined
	UserDraftDbSyncer.recordRemoteSync({ workspace, itemKind, path: storagePath }, resp.created_at)
	return resp.value ?? undefined
}

/** Draft VALUE at a key already resolved: cell-if-present (the user's freshest in-tab
 * edits) else the current user's backend draft. */
export async function readGlobalDraftValueAt<V>(
	workspace: string,
	type: WorkspaceItemType,
	storagePath: string,
	triggerKind?: TriggerKind
): Promise<V | undefined> {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const cell = UserDraft.get<V>(itemKind, storagePath, { workspace })
	if (cell !== undefined) return cell
	return (await fetchBackendDraftValue(workspace, itemKind, storagePath)) as V | undefined
}

// `itemKind` + `storagePath` are the canonical identity of the persisted draft
// (NOT item.path, which is the friendly display path). Callers use them to record
// the chat's modified-items mask.
export type DraftPersistResult =
	| { status: 'saved'; item: WorkspaceItem; itemKind: UserDraftItemKind; storagePath: string }
	| {
			status: 'conflict'
			item: WorkspaceItem
			itemKind: UserDraftItemKind
			storagePath: string
			serverTimestamp?: string
	  }
	| {
			status: 'error'
			item: WorkspaceItem
			itemKind: UserDraftItemKind
			storagePath: string
			message: string
	  }

// Persist a built draft value. `UserDraft.seed` reflects it into an open editor's
// cell WITHOUT a double-POST (no-ops if no cell; its seedNextWrite suppresses the
// cell's autosave mirror), then the awaited immediate save is the single source of
// persistence + conflict detection against the shared baseline. force overwrites.
export async function persistGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	value: unknown,
	opts: { triggerKind?: TriggerKind; force?: boolean; storagePath?: string } = {}
): Promise<DraftPersistResult> {
	const itemKind = itemKindFor(type, opts.triggerKind)
	if (!itemKind) throw new Error(`Unsupported draft type "${type}".`)
	// Resolved rather than taken as given: a write under a chosen name that did not resolve
	// would create a second draft beside the one it meant to edit. A caller that resolved
	// for the whole operation passes its key so both halves land on one draft.
	const storagePath =
		opts.storagePath ??
		(await resolveDraft(workspace, itemKind, path, { forWrite: true })).storagePath
	UserDraft.seed(itemKind, storagePath, value, { workspace })
	await UserDraftDbSyncer.save({
		workspace,
		itemKind,
		path: storagePath,
		value,
		immediate: true,
		force: opts.force
	})
	const { displayPath, isLiveDraft } = liveDisplayPath(workspace, itemKind, storagePath)
	const item = userDraftEntryToWorkspaceItem(
		{ workspace, itemKind, path: storagePath, value },
		displayPath,
		isLiveDraft
	)
	if (!item) throw new Error(`Could not synthesize ${type} draft "${path}".`)
	// A failed save (network/5xx) is recorded in the syncer's failure map, not
	// thrown — so check it before reporting success, else a write tool would tell
	// the chat "saved" while the DB-backed source of truth was never updated.
	const saveState = UserDraftDbSyncer.getState({ workspace, itemKind, path: storagePath })
	if (saveState.state === 'failed') {
		return {
			status: 'error',
			item,
			itemKind,
			storagePath,
			message: saveState.failureMessage ?? 'Draft save failed'
		}
	}
	const conflict = opts.force
		? undefined
		: UserDraftDbSyncer.getConflict({ workspace, itemKind, path: storagePath }).conflict
	if (conflict) {
		return {
			status: 'conflict',
			item,
			itemKind,
			storagePath,
			serverTimestamp: conflict.serverTimestamp
		}
	}
	invalidateWorkspaceDrafts(workspace)
	return { status: 'saved', item, itemKind, storagePath }
}

export async function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): Promise<WorkspaceItem | undefined> {
	const slot = getGlobalDraftSlot(workspace, type, path, triggerKind)
	if (slot) return slot.item
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const resolved = await resolveDraft(workspace, itemKind, path)
	const storagePath = resolved.storagePath
	// A draft can live only as a local cell — a second session tab, or an editor with
	// autosave off — with no backend row behind it.
	const value =
		UserDraft.get(itemKind, storagePath, { workspace }) ??
		resolved.value ??
		(await fetchBackendDraftValue(workspace, itemKind, storagePath))
	if (value === undefined || value === null) return undefined
	const { displayPath, isLiveDraft } = liveDisplayPath(workspace, itemKind, storagePath)
	return userDraftEntryToWorkspaceItem(
		{ workspace, itemKind, path: storagePath, value },
		displayPath,
		isLiveDraft
	)
}

// Maps a backend `listDrafts` metadata row (no value) to a lightweight item.
// The row's `path` is the storage path; remap it to the live editor's effective
// path (and flag it) when one is open on this key, matching the cell path.
function backendDraftRowToWorkspaceItem(
	workspace: string,
	row: {
		kind: string
		path: string
		summary?: string
		draft_path?: string
	}
): WorkspaceItem | undefined {
	if (!(GLOBAL_DRAFT_KINDS as readonly string[]).includes(row.kind)) return undefined
	let type: WorkspaceItemType
	let triggerKind: TriggerKind | undefined
	switch (row.kind) {
		case 'script':
		case 'flow':
		case 'resource':
		case 'variable':
			type = row.kind
			break
		case 'raw_app':
			type = 'app'
			break
		case 'trigger_schedule':
			type = 'schedule'
			break
		default: {
			const tk = TRIGGER_KIND_BY_DRAFT_KIND[row.kind as UserDraftItemKind]
			if (!tk) return undefined
			type = 'trigger'
			triggerKind = tk
		}
	}
	const { displayPath, isLiveDraft } = liveDisplayPath(
		workspace,
		row.kind as UserDraftItemKind,
		row.path
	)
	return {
		type,
		path: displayPath,
		// The row's friendly path (from the draft JSON) names the item; only a
		// draft_path that repeats the display path adds nothing. Kept even for
		// live rows — a live registration whose effective path is the storage key
		// (flow/raw-app renames live in the value's `draft_path`, not `path`)
		// must not hide the staged rename.
		draftPath: row.draft_path === displayPath ? undefined : row.draft_path,
		summary: row.summary,
		value: undefined,
		isDraft: true,
		triggerKind,
		rawApp: row.kind === 'raw_app' ? true : undefined,
		...(isLiveDraft ? { isLiveDraft: true } : {})
	}
}

export async function listGlobalDrafts(workspace: string): Promise<WorkspaceItem[]> {
	const drafts = new Map<string, WorkspaceItem>()
	const rows = await DraftService.listDrafts({ workspace })
	for (const row of rows) {
		const item = backendDraftRowToWorkspaceItem(workspace, row)
		if (!item) continue
		drafts.set(getWorkspaceItemKey(item.type, item.path, item.triggerKind), item)
	}
	// Overlay live in-tab cells (full values + the user's live edits); cell wins.
	for (const entry of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		const { displayPath, isLiveDraft } = liveDisplayPath(workspace, entry.itemKind, entry.path)
		const draft = userDraftEntryToWorkspaceItem(entry, displayPath, isLiveDraft)
		if (!draft) continue
		drafts.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), draft)
	}
	return Array.from(drafts.values())
}

export async function saveGlobalAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue
): Promise<DraftPersistResult> {
	// Return the full result (not just the item) so app write tools surface a
	// conflict / save failure instead of reporting every stale write as saved.
	return persistGlobalDraft(workspace, 'app', path, normalizeAppDraftValue(value), {})
}

type DeleteGlobalDraftOptions = {
	preserveLiveDraft?: boolean
	/** Key resolved by the caller, for a cleanup whose own resolution would now answer
	 * differently — after a deploy created the item at the draft's chosen name, or after
	 * the deployed item that outranked that name was deleted. */
	storagePath?: string
}

export async function deleteGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind,
	options: DeleteGlobalDraftOptions = {}
): Promise<void> {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return
	const storagePath =
		options.storagePath ?? (await resolveDraft(workspace, itemKind, path)).storagePath
	const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
	if (options.preserveLiveDraft && liveDraft?.storagePath === storagePath) {
		UserDraft.remove(itemKind, storagePath, { workspace })
	} else {
		UserDraft.clear(itemKind, storagePath, { workspace })
	}
	// `remove`/`clear` only debounce the delete; persist it now so a deploy/discard
	// that the caller awaits has actually cleared the server draft on return.
	await UserDraftDbSyncer.save({
		workspace,
		itemKind,
		path: storagePath,
		value: null,
		immediate: true
	})
	// A failed (network/5xx) or conflicted delete is recorded in the syncer state,
	// not thrown — surface it so callers don't report the draft as removed while
	// the DB-backed source of truth still has it (same guard as the write path).
	const state = UserDraftDbSyncer.getState({ workspace, itemKind, path: storagePath })
	if (state.state === 'failed') {
		throw new Error(state.failureMessage ?? `Failed to delete draft "${path}".`)
	}
	if (UserDraftDbSyncer.getConflict({ workspace, itemKind, path: storagePath }).conflict) {
		throw new Error(
			`Draft "${path}" changed externally since you last read it; it was not removed. Re-read and retry.`
		)
	}
	invalidateWorkspaceDrafts(workspace)
}

/** Kind-addressed live-editor storage resolution (friendly → storage path),
 * for callers that must probe several draft kinds per chat type. */
export function resolveGlobalDraftStoragePathByKind(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): string {
	return liveEditorStoragePath(workspace, itemKind, path)
}

/** Local in-memory draft cell, kind-addressed: the chat `app` type spans two
 * draft kinds (raw_app + classic app), so callers probing both address by
 * kind. No backend fallback — this is the freshest state when a save is
 * parked (auto-save off), failed, or conflicted; read-only callers use it
 * instead of persisting. */
export function readLocalDraftCellByKind(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): unknown | undefined {
	const storagePath = liveEditorStoragePath(workspace, itemKind, path)
	return UserDraft.get(itemKind, storagePath, { workspace })
}

/** Flush every parked local draft autosave for the workspace so the server
 * listing reflects the latest edits — a brand-new editor draft has no server
 * row until its first flush. No-ops per key when nothing is pending.
 * Honors the auto-save toggle: with auto-save off, parked editor edits stay
 * parked — a read-only caller must not persist what the user chose not to.
 * `unflushedPaths` lists items whose latest edits did NOT reach the server
 * (toggle-parked or failed save), so callers can say the listing excludes them. */
export async function flushGlobalDraftSaves(
	workspace: string
): Promise<{ unflushedPaths: string[] }> {
	// Classic-app editor cells live under the `app` kind, which is deliberately
	// NOT in GLOBAL_DRAFT_KINDS (clearGlobalDrafts must never clear a user's
	// open classic editor) — but their unflushed edits must be flushed/reported
	// like every other kind.
	const drafts = UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS, 'app'] })
	await Promise.all(
		drafts.map((draft) =>
			UserDraftDbSyncer.flush(
				{ workspace, itemKind: draft.itemKind, path: draft.path },
				{ honorAutosaveToggle: true }
			)
		)
	)
	const unflushedPaths = drafts
		.filter((draft) => {
			const query = { workspace, itemKind: draft.itemKind, path: draft.path }
			// A conflicted save also leaves the server without the local edits:
			// the payload stays parked but the state is neither pending nor failed.
			return (
				UserDraftDbSyncer.hasUnsavedDisabledChanges(query) ||
				UserDraftDbSyncer.getState(query).state === 'failed' ||
				UserDraftDbSyncer.getConflict(query).conflict !== undefined
			)
		})
		.map((draft) => draft.path)
	return { unflushedPaths }
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		UserDraft.clear(draft.itemKind, draft.path, { workspace })
	}
}
