import type { Flow, NewSchedule, NewScript } from '$lib/gen/types.gen'
import { ApiError, DraftService } from '$lib/gen'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import {
	UserDraft,
	type UserDraftEntry,
	type UserDraftItemKind,
	type UserDraftMeta
} from '$lib/userDraft.svelte'
import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
import {
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
		custom_path: value.custom_path
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

function itemKindFor(
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
		isDraft: true
	}
}

function flowDraftToWorkspaceItem(path: string, draft: Flow): WorkspaceItem {
	return {
		type: 'flow',
		path,
		summary: draft.summary,
		value: {
			value: draft.value,
			schema: draft.schema ?? null,
			groups: draft.value.groups ?? null
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
	// Fall back to the caller's path when there's no live editor, or when
	// the live editor hasn't committed a storage path yet. An empty
	// storage path must never propagate to the DB seam: it can't be a
	// `draft` row key and the draft routes (`/save_draft/{kind}/{*path}`)
	// 404 on an empty wildcard tail. In practice new drafts always live at
	// a real `u/{user}/draft_{uuid}` path (see `/scripts/add` et al.), so
	// this is defensive — but it keeps `''` out of `DraftService` outright.
	if (!liveDraft || !liveDraft.storagePath) return path
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

function isNotFoundError(e: unknown): boolean {
	return e instanceof ApiError && e.status === 404
}

/**
 * Raw draft value for `(workspace, itemKind, storagePath)`. The in-tab
 * mounted cell wins — it's the freshest, holding any unsaved live-editor
 * edits that haven't been flushed to the server yet — otherwise the
 * per-user DB draft. Returns `undefined` when no draft exists in either
 * place. Async because the headless fallback is a fetch.
 *
 * Post #9351 the global mode is headless (never mounts a handle), so the
 * in-memory branch only hits when an editor is open on the same item; the
 * DB branch is the common one and is what makes a draft written in a
 * previous turn / tab readable here.
 */
export async function readGlobalDraftValue<V>(
	workspace: string,
	itemKind: UserDraftItemKind,
	storagePath: string
): Promise<V | undefined> {
	const local = UserDraft.get<V>(itemKind, storagePath, { workspace })
	if (local !== undefined) return local
	try {
		const resp = await DraftService.getDraft({ workspace, kind: itemKind, path: storagePath })
		return resp.value as V
	} catch (e) {
		if (isNotFoundError(e)) return undefined
		throw e
	}
}

/**
 * Persist a draft value for `(workspace, itemKind, storagePath)`.
 *
 * When an editor handle is mounted for this entry, route through the
 * in-memory `UserDraft` layer so the open editor reflects the write
 * reactively (its background syncer still persists it). When headless —
 * the common path for AI-driven writes — push straight to the DB and
 * AWAIT it, so a read-back immediately after sees the value: this closes
 * the read-after-write gap (`UserDraft.save` alone never seeds the in-tab
 * cell for an unmounted entry, so a subsequent `UserDraft.get` would
 * return `undefined`). Forced — AI overwrites have no human at a conflict
 * modal. Rev metadata is in-memory only and is dropped on the headless
 * path (the DB stores values, not revs); the next editor mount reseeds it.
 */
export async function saveGlobalDraftValue<V>(
	workspace: string,
	itemKind: UserDraftItemKind,
	storagePath: string,
	value: V,
	meta?: UserDraftMeta
): Promise<void> {
	if (UserDraft.isLive(itemKind, storagePath, { workspace })) {
		if (meta) {
			UserDraft.setDraftAndMeta(itemKind, storagePath, value, meta, { workspace })
		} else {
			UserDraft.save(itemKind, storagePath, value, { workspace })
		}
		return
	}
	await UserDraftDbSyncer.save({
		workspace,
		itemKind,
		path: storagePath,
		value,
		immediate: true,
		force: true,
		// Headless write: a failed POST must reject so the calling tool
		// reports the failure, instead of being silently swallowed and
		// then read back as a stale value (or 404) and mis-reported as a
		// successful write.
		throwOnError: true
	})
}

/**
 * Shape a draft `value` into a `WorkspaceItem` at its live-editor display
 * path. Shared by the DB read path (`getGlobalDraftSlot`) and the
 * write-then-shape path (`shapeGlobalDraftItem`) so both yield an
 * identical item for the same value.
 */
function buildGlobalDraftItem<V>(
	workspace: string,
	itemKind: UserDraftItemKind,
	storagePath: string,
	value: V
): { displayPath: string; item: WorkspaceItem } | undefined {
	const { displayPath, isLiveDraft } = liveDisplayPath(workspace, itemKind, storagePath)
	const entry: UserDraftEntry = {
		workspace,
		itemKind,
		path: storagePath,
		value,
		meta: {}
	}
	const item = userDraftEntryToWorkspaceItem(entry, displayPath, isLiveDraft)
	return item ? { displayPath, item } : undefined
}

/**
 * Shape a freshly-written draft `value` into a `WorkspaceItem` WITHOUT a
 * server read-back. The value is exactly what `saveGlobalDraftValue` just
 * persisted (and that save now rejects on failure), so reading it back
 * would be a redundant round trip — and, before the syncer learned to
 * reject failed writes, the read-back could mask a failed save by
 * returning the stale server copy. Mirrors `getGlobalDraftSlot`'s shaping
 * (same live-editor display-path / `isLiveDraft` resolution), sourced
 * from the in-hand value.
 */
export function shapeGlobalDraftItem<V>(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	value: V,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
	return buildGlobalDraftItem(workspace, itemKind, storagePath, value)?.item
}

async function getGlobalDraftSlot(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
) {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
	const draft = await readGlobalDraftValue(workspace, itemKind, storagePath)
	if (draft === undefined) return undefined

	const shaped = buildGlobalDraftItem(workspace, itemKind, storagePath, draft)
	if (!shaped) return undefined
	return { itemKind, storagePath, displayPath: shaped.displayPath, item: shaped.item }
}

export async function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): Promise<WorkspaceItem | undefined> {
	return (await getGlobalDraftSlot(workspace, type, path, triggerKind))?.item
}

const LIVE_EDITOR_DRAFT_KINDS = [
	'script',
	'flow',
	'raw_app'
] as const satisfies readonly UserDraftItemKind[]

function liveEditorDraftType(kind: (typeof LIVE_EDITOR_DRAFT_KINDS)[number]): WorkspaceItemType {
	return kind === 'raw_app' ? 'app' : kind
}

/**
 * The open editor's in-flight draft for each editor kind (script/flow/raw_app)
 * as a value-less `WorkspaceItem` at its *effective* path, flagged
 * `isLiveDraft`.
 *
 * This is in-memory state that the DB draft list can't represent: a brand-new
 * draft that hasn't been saved to the server yet has no DB row to list, and an
 * in-progress rename's effective path differs from the `u/{user}/draft_{uuid}`
 * path where the draft is stored. Persisted draft listing comes from the
 * backend (`includeDraftOnly` + `isDraft`); this only fills that in-memory gap.
 *
 * Existence and path are taken straight from the live registry
 * (`getLiveEditorDraft`) — not gated on the in-tab value cell, which is only
 * populated while the editor's handle is mounted. `summary` is best-effort
 * from that cell when present. `storagePath` is returned so the caller can drop
 * a stale list entry at the pre-rename path.
 */
export function listLiveEditorDrafts(
	workspace: string
): { item: WorkspaceItem; storagePath: string }[] {
	const out: { item: WorkspaceItem; storagePath: string }[] = []
	for (const itemKind of LIVE_EDITOR_DRAFT_KINDS) {
		const live = UserDraft.getLiveEditorDraft(itemKind, { workspace })
		if (!live) continue
		const path = live.effectivePath || live.storagePath
		if (!path) continue
		const value = UserDraft.get(itemKind, live.storagePath, { workspace })
		out.push({
			item: {
				type: liveEditorDraftType(itemKind),
				path,
				summary: getItemSummary(value),
				isDraft: true,
				isLiveDraft: true
			},
			storagePath: live.storagePath
		})
	}
	return out
}

export async function saveGlobalAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue,
	meta?: UserDraftMeta
): Promise<WorkspaceItem> {
	const storagePath = resolveDraftStoragePath(workspace, 'raw_app', path)
	const normalized = normalizeAppDraftValue(value)
	await saveGlobalDraftValue(workspace, 'raw_app', storagePath, normalized, meta)
	// Shape from the value we just persisted instead of reading it back —
	// the save above rejects on failure, so a read-back would only add a
	// round trip (and risk returning a stale copy).
	const stored = shapeGlobalDraftItem(workspace, 'app', path, normalized)
	if (!stored) throw new Error(`Could not shape written app draft "${path}".`)
	return stored
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
	if (UserDraft.isLive(itemKind, storagePath, { workspace })) {
		// An editor is open on this draft: reset its in-memory cell so the
		// UI reflects the delete. `remove`/`clear` also queue a *debounced*
		// `value: null` sync, which the awaited immediate delete below
		// supersedes — `immediate` cancels the queued debouncer task, so
		// exactly one delete lands.
		const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
		if (options.preserveLiveDraft && liveDraft?.storagePath === storagePath) {
			UserDraft.remove(itemKind, storagePath, { workspace })
		} else {
			UserDraft.clear(itemKind, storagePath, { workspace })
		}
	}
	// Always delete the DB row with an awaited, immediate, forced write. The
	// live branch above only clears the in-tab cell and queues a *debounced*
	// null sync; without this, an immediate read-back would fall through
	// `readGlobalDraftValue`'s empty-cell check to `DraftService.getDraft` and
	// resurrect the still-present DB row until the debounce flushed.
	await UserDraftDbSyncer.save({
		workspace,
		itemKind,
		path: storagePath,
		value: null,
		immediate: true,
		force: true,
		// A failed delete must reject so callers (discard / deploy /
		// delete tools) don't report "discarded" while the row survives.
		throwOnError: true
	})
	if (type === 'variable') clearEphemeralSecretVariableDraftValue(workspace, storagePath)
}

export async function clearGlobalDrafts(workspace: string): Promise<void> {
	// `UserDraft.list` only sees in-tab mounted entries post #9351, so
	// enumerate the user's persisted drafts from the DB instead and delete
	// each global-mode kind.
	const kinds = new Set<UserDraftItemKind>(GLOBAL_DRAFT_KINDS)
	const drafts = await DraftService.listDrafts({ workspace })
	for (const draft of drafts) {
		if (!kinds.has(draft.typ)) continue
		await UserDraftDbSyncer.save({
			workspace,
			itemKind: draft.typ,
			path: draft.path,
			value: null,
			immediate: true,
			force: true,
			throwOnError: true
		})
	}
	clearEphemeralSecretVariableDraftValues(workspace)
}
