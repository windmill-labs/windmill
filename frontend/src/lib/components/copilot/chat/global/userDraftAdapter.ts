import type {
	CreateResource,
	CreateVariable,
	FlowValue,
	NewSchedule,
	NewScript
} from '$lib/gen/types.gen'
import {
	UserDraft,
	type UserDraftEntrySource,
	type UserDraftItemKind,
	type UserDraftListEntry
} from '$lib/userDraft.svelte'
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
			return scriptDraftToWorkspaceItem(entry.storagePath, entry.value as NewScript, isDraft)
		case 'flow':
			return flowDraftToWorkspaceItem(entry.storagePath, entry.value, isDraft)
		case 'raw_app':
			return appDraftToWorkspaceItem(entry.storagePath, entry.value as AppDraftValue, isDraft)
		case 'trigger_schedule':
			return scheduleDraftToWorkspaceItem(entry.storagePath, entry.value as NewSchedule, isDraft)
		case 'resource':
			return resourceDraftToWorkspaceItem(entry.storagePath, entry.value as CreateResource, isDraft)
		case 'variable':
			return variableDraftToWorkspaceItem(entry.storagePath, entry.value as CreateVariable, isDraft)
		default:
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[entry.itemKind]
			return triggerKind
				? triggerDraftToWorkspaceItem(
						triggerKind,
						entry.storagePath,
						entry.value as TriggerRequestBody,
						isDraft
					)
				: undefined
	}
}

type LocalWorkspaceItemLookup = {
	storagePath: string
	source: UserDraftEntrySource
	draftSource?: UserDraftListEntry['draftSource']
	item: WorkspaceItem
}

function isDeployableEntry(entry: Pick<LocalWorkspaceItemLookup, 'source' | 'draftSource'>): boolean {
	return entry.source !== 'live' && entry.draftSource === 'external'
}

function itemHasTarget(
	item: WorkspaceItem,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): boolean {
	if (item.type !== type) return false
	if (item.path !== path) return false
	return type !== 'trigger' || item.triggerKind === triggerKind
}

function listSharedEntries(
	workspace: string,
	itemKinds: UserDraftItemKind[],
	isDraft: boolean
): LocalWorkspaceItemLookup[] {
	const entries: LocalWorkspaceItemLookup[] = []
	for (const entry of UserDraft.list({ workspace, itemKinds })) {
		const item = sharedDraftEntryToWorkspaceItem(entry, isDraft)
		if (!item) continue
		entries.push({
			storagePath: entry.storagePath,
			source: entry.source,
			draftSource: entry.draftSource,
			item
		})
	}
	return entries
}

function findSharedEntry(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind: TriggerKind | undefined,
	mode: 'draft' | 'current'
): LocalWorkspaceItemLookup | undefined {
	const itemKind = sharedDraftKind(type, triggerKind)
	if (!itemKind) return undefined

	for (const entry of listSharedEntries(workspace, [itemKind], mode === 'draft')) {
		if (mode === 'draft' && !isDeployableEntry(entry)) continue
		if (itemHasTarget(entry.item, type, path, triggerKind)) return entry
	}

	return undefined
}

function getSharedItem(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind,
	mode: 'draft' | 'current' = 'draft'
): WorkspaceItem | undefined {
	return findSharedEntry(workspace, type, path, triggerKind, mode)?.item
}

function deleteSharedDraft(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): void {
	const itemKind = sharedDraftKind(type, triggerKind)
	if (!itemKind) return
	const draft = findSharedEntry(workspace, type, path, triggerKind, 'current')
	UserDraft.clear(itemKind, draft?.storagePath ?? path, { workspace })
}

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	if (isSharedWorkspaceItemType(type)) {
		return getSharedItem(workspace, type, path, triggerKind, 'draft')
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
		return getSharedItem(workspace, type, path, triggerKind, 'current')
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
		return findSharedEntry(workspace, type, path, triggerKind, 'current')?.storagePath
	}
	return undefined
}

export function listGlobalDrafts(workspace: string): WorkspaceItem[] {
	const drafts = new Map<string, WorkspaceItem>()

	for (const entry of listSharedEntries(workspace, [...SHARED_DRAFT_KINDS], true)) {
		if (!isDeployableEntry(entry)) continue
		const { item: draft } = entry
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

	for (const { item } of listSharedEntries(
		workspace,
		sharedDraftKindsForTypes(types),
		false
	)) {
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
		if (!isDeployableEntry(draft)) continue
		UserDraft.clear(draft.itemKind, draft.storagePath, { workspace })
	}
}

export function triggerKindToUserDraftKind(kind: TriggerKind): UserDraftItemKind {
	return TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[kind]
}
