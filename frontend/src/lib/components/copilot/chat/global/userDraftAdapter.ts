import type { Flow, NewSchedule, NewScript } from '$lib/gen/types.gen'
import { DraftService } from '$lib/gen'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import { UserDraft, type UserDraftEntry, type UserDraftItemKind } from '$lib/userDraft.svelte'
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
	sqs: 'trigger_sqs',
	gcp: 'trigger_gcp',
	azure: 'trigger_azure'
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
	'trigger_sqs',
	'trigger_gcp',
	'trigger_azure',
	'resource',
	'variable'
] as const satisfies UserDraftItemKind[]

const secretVariableDraftValues = new Map<string, Map<string, string>>()

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
		parent_version: value.parent_version
	}
}

function getItemSummary(value: unknown): string | undefined {
	return ((value as { summary?: string | null } | undefined)?.summary ?? undefined) || undefined
}

export function setEphemeralSecretVariableDraftValue(
	workspace: string,
	path: string,
	value: string
): void {
	let workspaceValues = secretVariableDraftValues.get(workspace)
	if (!workspaceValues) {
		workspaceValues = new Map()
		secretVariableDraftValues.set(workspace, workspaceValues)
	}
	workspaceValues.set(path, value)
}

export function getEphemeralSecretVariableDraftValue(
	workspace: string,
	path: string
): string | undefined {
	return secretVariableDraftValues.get(workspace)?.get(path)
}

export function clearEphemeralSecretVariableDraftValue(workspace: string, path: string): void {
	const workspaceValues = secretVariableDraftValues.get(workspace)
	if (!workspaceValues) return
	workspaceValues.delete(path)
	if (workspaceValues.size === 0) secretVariableDraftValues.delete(workspace)
}

function clearEphemeralSecretVariableDraftValues(workspace: string): void {
	secretVariableDraftValues.delete(workspace)
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

function scriptDraftToWorkspaceItem(path: string, draft: NewScript): WorkspaceItem {
	return {
		type: 'script',
		path,
		summary: draft.summary,
		language: draft.language,
		value: draft.content,
		parentHash: draft.parent_hash,
		isDraft: true
	}
}

function flowDraftToWorkspaceItem(path: string, draft: Flow): WorkspaceItem {
	return {
		type: 'flow',
		path,
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
		summary: value.summary,
		parentVersionId: value.parent_version,
		value,
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
	return item && isLiveDraft ? { ...item, isLiveDraft: true } : item
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

function resolveDraftStoragePath(
	workspace: string,
	itemKind: UserDraftItemKind,
	path: string
): string {
	const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
	if (!liveDraft) return path
	if (path === liveDraft.storagePath || path === liveDraft.effectivePath)
		return liveDraft.storagePath
	return path
}

export function getGlobalDraftStoragePath(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): string {
	const itemKind = itemKindFor(type, triggerKind)
	return itemKind ? resolveDraftStoragePath(workspace, itemKind, path) : path
}

function getGlobalDraftSlot(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
) {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
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

// Draft VALUE for a write merge: cell-if-present (the user's freshest in-tab
// edits) else the current user's backend draft.
export async function readGlobalDraftValue<V>(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): Promise<V | undefined> {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
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
	opts: { triggerKind?: TriggerKind; force?: boolean } = {}
): Promise<DraftPersistResult> {
	const itemKind = itemKindFor(type, opts.triggerKind)
	if (!itemKind) throw new Error(`Unsupported draft type "${type}".`)
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
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
	return conflict
		? { status: 'conflict', item, itemKind, storagePath, serverTimestamp: conflict.serverTimestamp }
		: { status: 'saved', item, itemKind, storagePath }
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
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
	const value = await fetchBackendDraftValue(workspace, itemKind, storagePath)
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
		summary: row.summary,
		value: undefined,
		isDraft: true,
		triggerKind,
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
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
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
	if (type === 'variable') clearEphemeralSecretVariableDraftValue(workspace, storagePath)
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		UserDraft.clear(draft.itemKind, draft.path, { workspace })
	}
	clearEphemeralSecretVariableDraftValues(workspace)
}
