import type {
	CreateResource,
	CreateVariable,
	FlowValue,
	NewSchedule,
	NewScript
} from '$lib/gen/types.gen'
import { UserDraft, type UserDraftItemKind, type UserDraftListEntry } from '$lib/userDraft.svelte'
import {
	getWorkspaceItemKey,
	type AppDraftValue,
	type TriggerRequestBody,
	type TriggerKind,
	type WorkspaceItem,
	type WorkspaceItemType
} from './workspaceItems'

type SharedWorkspaceItemType =
	| 'script'
	| 'flow'
	| 'app'
	| 'schedule'
	| 'trigger'
	| 'resource'
	| 'variable'

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

const TRIGGER_KIND_BY_DRAFT_KIND = {
	trigger_http: 'http',
	trigger_websocket: 'websocket',
	trigger_kafka: 'kafka',
	trigger_nats: 'nats',
	trigger_postgres: 'postgres',
	trigger_mqtt: 'mqtt',
	trigger_sqs: 'sqs',
	trigger_gcp: 'gcp',
	trigger_azure: 'azure'
} as const satisfies Partial<Record<UserDraftItemKind, TriggerKind>>

const SHARED_DRAFT_KINDS = [
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
const DEFAULT_APP_DATA = { tables: [], datatable: undefined, schema: undefined }
const storagePathByVisibleItem = new Map<string, string>()

function clone<T>(value: T): T {
	return structuredClone(value) as T
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function isSharedWorkspaceItemType(type: WorkspaceItemType): type is SharedWorkspaceItemType {
	return (
		type === 'script' ||
		type === 'flow' ||
		type === 'app' ||
		type === 'schedule' ||
		type === 'trigger' ||
		type === 'resource' ||
		type === 'variable'
	)
}

function sharedDraftKind(
	type: SharedWorkspaceItemType,
	triggerKind?: TriggerKind
): UserDraftItemKind | undefined {
	switch (type) {
		case 'script':
		case 'flow':
			return type
		case 'app':
			return 'raw_app'
		case 'schedule':
			return 'trigger_schedule'
		case 'trigger':
			return triggerKind ? TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[triggerKind] : undefined
		case 'resource':
		case 'variable':
			return type
	}
}

function sharedDraftKindsForTypes(types?: readonly WorkspaceItemType[]): UserDraftItemKind[] {
	if (!types) return [...SHARED_DRAFT_KINDS]

	const itemKinds = new Set<UserDraftItemKind>()
	for (const type of types) {
		switch (type) {
			case 'script':
			case 'flow':
			case 'resource':
			case 'variable':
				itemKinds.add(type)
				break
			case 'app':
				itemKinds.add('raw_app')
				break
			case 'schedule':
				itemKinds.add('trigger_schedule')
				break
			case 'trigger':
				for (const itemKind of Object.values(TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND)) {
					itemKinds.add(itemKind)
				}
				break
		}
	}

	return [...itemKinds]
}

function normalizeAppDraftValue(value: AppDraftValue): AppDraftValue {
	return {
		...value,
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: value.data ?? { ...DEFAULT_APP_DATA }
	}
}

function getItemSummary(value: unknown): string | undefined {
	return ((value as { summary?: string | null } | undefined)?.summary ?? undefined) || undefined
}

function storagePathCacheKey(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): string {
	return `${workspace}:${getWorkspaceItemKey(type, path, triggerKind)}`
}

function rememberStoragePath(workspace: string, item: WorkspaceItem, storagePath: string): void {
	storagePathByVisibleItem.set(
		storagePathCacheKey(workspace, item.type, item.path, item.triggerKind),
		storagePath
	)
}

function forgetStoragePath(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): void {
	storagePathByVisibleItem.delete(storagePathCacheKey(workspace, type, path, triggerKind))
}

function getItemPath(storagePath: string, value: unknown): string | undefined {
	const valuePath = (value as { path?: string | null } | undefined)?.path
	return valuePath?.trim() || storagePath || undefined
}

function scriptDraftToWorkspaceItem(
	storagePath: string,
	draft: NewScript,
	isDraft = true
): WorkspaceItem | undefined {
	const path = getItemPath(storagePath, draft)
	if (!path) return undefined

	return {
		type: 'script',
		path,
		summary: draft.summary,
		language: draft.language,
		value: draft.content,
		isDraft
	}
}

function getFlowValue(value: unknown): FlowValue | undefined {
	if (!isRecord(value) || !Array.isArray(value.modules)) return undefined
	return clone(value as FlowValue)
}

function getFlowGroups(value: FlowValue): FlowValue['groups'] | null {
	return Array.isArray(value.groups) ? clone(value.groups) : null
}

function getFlowSchema(value: unknown): Record<string, any> | null {
	return isRecord(value) ? clone(value as Record<string, any>) : null
}

function flowDraftToWorkspaceItem(
	storagePath: string,
	draft: unknown,
	isDraft = true
): WorkspaceItem | undefined {
	if (!isRecord(draft)) return undefined

	const value = getFlowValue(draft.value) ?? getFlowValue(draft)
	if (!value) return undefined
	const path = getItemPath(storagePath, draft)
	if (!path) return undefined

	return {
		type: 'flow',
		path,
		summary: typeof draft.summary === 'string' ? draft.summary : undefined,
		value: {
			value,
			schema: getFlowSchema(draft.schema),
			groups: Array.isArray(draft.groups)
				? clone(draft.groups as FlowValue['groups'])
				: getFlowGroups(value)
		},
		isDraft
	}
}

function appDraftToWorkspaceItem(
	storagePath: string,
	draft: AppDraftValue,
	isDraft = true
): WorkspaceItem | undefined {
	const value = normalizeAppDraftValue(draft)
	const path = getItemPath(storagePath, value)
	if (!path) return undefined

	return {
		type: 'app',
		path,
		summary: value.summary,
		value,
		isDraft
	}
}

function scheduleDraftToWorkspaceItem(
	storagePath: string,
	draft: NewSchedule,
	isDraft = true
): WorkspaceItem | undefined {
	const path = getItemPath(storagePath, draft)
	if (!path) return undefined

	return {
		type: 'schedule',
		path,
		summary: draft.summary ?? undefined,
		value: clone(draft),
		isDraft
	}
}

function triggerDraftToWorkspaceItem(
	kind: TriggerKind,
	storagePath: string,
	draft: TriggerRequestBody,
	isDraft = true
): WorkspaceItem | undefined {
	const path = getItemPath(storagePath, draft)
	if (!path) return undefined

	return {
		type: 'trigger',
		triggerKind: kind,
		path,
		summary: getItemSummary(draft),
		value: clone(draft),
		isDraft
	}
}

function resourceDraftToWorkspaceItem(
	storagePath: string,
	draft: CreateResource,
	isDraft = true
): WorkspaceItem | undefined {
	const path = getItemPath(storagePath, draft)
	if (!path) return undefined

	return {
		type: 'resource',
		path,
		summary: draft.description ?? undefined,
		value: clone(draft),
		isDraft
	}
}

function variableDraftToWorkspaceItem(
	storagePath: string,
	draft: CreateVariable,
	isDraft = true
): WorkspaceItem | undefined {
	const path = getItemPath(storagePath, draft)
	if (!path) return undefined

	return {
		type: 'variable',
		path,
		summary: draft.description ?? undefined,
		value: clone(draft),
		isDraft
	}
}

function sharedDraftEntryToWorkspaceItem(
	entry: UserDraftListEntry,
	isDraft = true
): WorkspaceItem | undefined {
	switch (entry.itemKind) {
		case 'script':
			return scriptDraftToWorkspaceItem(entry.path, entry.value as NewScript, isDraft)
		case 'flow':
			return flowDraftToWorkspaceItem(entry.path, entry.value, isDraft)
		case 'raw_app':
			return appDraftToWorkspaceItem(entry.path, entry.value as AppDraftValue, isDraft)
		case 'trigger_schedule':
			return scheduleDraftToWorkspaceItem(entry.path, entry.value as NewSchedule, isDraft)
		case 'resource':
			return resourceDraftToWorkspaceItem(entry.path, entry.value as CreateResource, isDraft)
		case 'variable':
			return variableDraftToWorkspaceItem(entry.path, entry.value as CreateVariable, isDraft)
		default:
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[entry.itemKind]
			return triggerKind
				? triggerDraftToWorkspaceItem(
						triggerKind,
						entry.path,
						entry.value as TriggerRequestBody,
						isDraft
					)
				: undefined
	}
}

type SharedDraftLookup = {
	storagePath: string
	value: unknown
}

function findSharedDraft(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): SharedDraftLookup | undefined {
	const itemKind = sharedDraftKind(type, triggerKind)
	if (!itemKind) return undefined

	const cachedStoragePath = storagePathByVisibleItem.get(
		storagePathCacheKey(workspace, type, path, triggerKind)
	)
	if (cachedStoragePath !== undefined) {
		const cached = UserDraft.get(itemKind, cachedStoragePath, { workspace })
		if (cached !== undefined) return { storagePath: cachedStoragePath, value: cached }
		forgetStoragePath(workspace, type, path, triggerKind)
	}

	const direct = UserDraft.get(itemKind, path, { workspace })
	if (direct !== undefined) return { storagePath: path, value: direct }

	if (path !== '') {
		const addEditorDraft = UserDraft.get(itemKind, '', { workspace })
		if (addEditorDraft !== undefined && getItemPath('', addEditorDraft) === path) {
			return { storagePath: '', value: addEditorDraft }
		}
	}

	for (const entry of UserDraft.list({ workspace, itemKinds: [itemKind] })) {
		if (getItemPath(entry.path, entry.value) === path) {
			return { storagePath: entry.path, value: entry.value }
		}
	}

	return undefined
}

function getSharedDraft(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind,
	isDraft = true
): WorkspaceItem | undefined {
	const draft = findSharedDraft(workspace, type, path, triggerKind)
	if (!draft) return undefined

	switch (type) {
		case 'script':
			return scriptDraftToWorkspaceItem(draft.storagePath, draft.value as NewScript, isDraft)
		case 'flow':
			return flowDraftToWorkspaceItem(draft.storagePath, draft.value, isDraft)
		case 'app':
			return appDraftToWorkspaceItem(draft.storagePath, draft.value as AppDraftValue, isDraft)
		case 'schedule':
			return scheduleDraftToWorkspaceItem(draft.storagePath, draft.value as NewSchedule, isDraft)
		case 'trigger':
			return triggerKind
				? triggerDraftToWorkspaceItem(
						triggerKind,
						draft.storagePath,
						draft.value as TriggerRequestBody,
						isDraft
					)
				: undefined
		case 'resource':
			return resourceDraftToWorkspaceItem(draft.storagePath, draft.value as CreateResource, isDraft)
		case 'variable':
			return variableDraftToWorkspaceItem(draft.storagePath, draft.value as CreateVariable, isDraft)
	}
}

function deleteSharedDraft(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): void {
	const itemKind = sharedDraftKind(type, triggerKind)
	if (!itemKind) return
	const draft = findSharedDraft(workspace, type, path, triggerKind)
	UserDraft.remove(itemKind, draft?.storagePath ?? path, { workspace })
}

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	if (isSharedWorkspaceItemType(type)) {
		return getSharedDraft(workspace, type, path, triggerKind)
	}
	return undefined
}

/**
 * Returns the freshest frontend-visible value for AI context. This may be a
 * real local draft, an AI-written UserDraft, or a clean live editor baseline.
 * Therefore the returned item is intentionally not marked as `isDraft`.
 */
export function getGlobalCurrentItem(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	if (isSharedWorkspaceItemType(type)) {
		return getSharedDraft(workspace, type, path, triggerKind, false)
	}
	return undefined
}

export function getGlobalDraftStoragePath(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): string | undefined {
	if (isSharedWorkspaceItemType(type)) {
		return findSharedDraft(workspace, type, path, triggerKind)?.storagePath
	}
	return undefined
}

export function listGlobalDrafts(workspace: string): WorkspaceItem[] {
	const drafts = new Map<string, WorkspaceItem>()

	for (const entry of UserDraft.list({ workspace, itemKinds: [...SHARED_DRAFT_KINDS] })) {
		const draft = sharedDraftEntryToWorkspaceItem(entry)
		if (!draft) continue
		rememberStoragePath(workspace, draft, entry.path)
		drafts.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), draft)
	}

	return Array.from(drafts.values())
}

/**
 * Lists local/current entries for AI context. Entries are not marked as
 * `isDraft` because UserDraft.list can include clean live editor baselines.
 */
export function listGlobalCurrentItems(
	workspace: string,
	types?: readonly WorkspaceItemType[]
): WorkspaceItem[] {
	const items = new Map<string, WorkspaceItem>()

	for (const entry of UserDraft.list({
		workspace,
		itemKinds: sharedDraftKindsForTypes(types)
	})) {
		const item = sharedDraftEntryToWorkspaceItem(entry, false)
		if (!item) continue
		rememberStoragePath(workspace, item, entry.path)
		items.set(getWorkspaceItemKey(item.type, item.path, item.triggerKind), item)
	}

	return Array.from(items.values())
}

export function deleteGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): void {
	if (isSharedWorkspaceItemType(type)) {
		deleteSharedDraft(workspace, type, path, triggerKind)
	}
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...SHARED_DRAFT_KINDS] })) {
		UserDraft.remove(draft.itemKind, draft.path, { workspace })
	}
}

export function triggerKindToUserDraftKind(kind: TriggerKind): UserDraftItemKind {
	return TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[kind]
}
