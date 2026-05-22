import type {
	CreateResource,
	CreateVariable,
	Flow,
	NewSchedule,
	NewScript,
	ScriptLang
} from '$lib/gen/types.gen'
import { UserDraft, type UserDraftEntry, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { emptySchema } from '$lib/utils'
import {
	getWorkspaceItemKey,
	type AppDraftValue,
	type FlowDraftValue,
	type TriggerKind,
	type TriggerRequestBody,
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

const DEFAULT_SCRIPT_LANGUAGE: ScriptLang = 'bun'
const DEFAULT_APP_DATA = { tables: [], datatable: undefined, schema: undefined }

export type GlobalDraftSlot<T = unknown> = {
	itemKind: UserDraftItemKind
	storagePath: string
	value: T
	item: WorkspaceItem
}

type ResourceDraftState = {
	path: string
	description: string
	args: Record<string, any>
	labels: string[] | undefined
	wsSpecific: boolean
	resource_type?: string
}

type VariableDraftState = {
	path: string
	variable: { value: string; is_secret: boolean; description: string }
	labels: string[] | undefined
	wsSpecific: boolean
	account?: number
	is_oauth?: boolean
	expires_at?: string
}

function clone<T>(value: T): T {
	return structuredClone(value) as T
}

function normalizeAppDraftValue(value: AppDraftValue): AppDraftValue {
	return {
		summary: value.summary,
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: value.data ?? { ...DEFAULT_APP_DATA }
	}
}

function getItemSummary(value: unknown): string | undefined {
	return ((value as { summary?: string | null } | undefined)?.summary ?? undefined) || undefined
}

function pathFromDraftValue(value: unknown): string | undefined {
	const path = (value as { path?: unknown } | undefined)?.path
	return typeof path === 'string' && path.length > 0 ? path : undefined
}

function effectiveDraftPath(entry: UserDraftEntry): string {
	return pathFromDraftValue(entry.value) ?? entry.path
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

function userDraftEntryToWorkspaceItem(entry: UserDraftEntry): WorkspaceItem | undefined {
	const path = effectiveDraftPath(entry)
	if (!path) return undefined

	switch (entry.itemKind) {
		case 'script':
			return scriptDraftToWorkspaceItem(path, entry.value as NewScript)
		case 'flow':
			return flowDraftToWorkspaceItem(path, entry.value as Flow)
		case 'raw_app':
			return appDraftToWorkspaceItem(path, entry.value as AppDraftValue)
		case 'trigger_schedule':
			return scheduleDraftToWorkspaceItem(path, entry.value as NewSchedule)
		case 'resource':
			return resourceDraftToWorkspaceItem(path, entry.value as ResourceDraftState)
		case 'variable':
			return variableDraftToWorkspaceItem(path, entry.value as VariableDraftState)
		default: {
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[entry.itemKind]
			return triggerKind
				? triggerDraftToWorkspaceItem(triggerKind, path, entry.value as TriggerRequestBody)
				: undefined
		}
	}
}

function scriptItemToDraft(item: WorkspaceItem): NewScript {
	if (typeof item.value !== 'string') {
		throw new Error(`Draft script "${item.path}" is missing source content.`)
	}
	return {
		path: item.path,
		summary: item.summary ?? '',
		description: '',
		content: item.value,
		schema: emptySchema(),
		is_template: false,
		language: item.language ?? DEFAULT_SCRIPT_LANGUAGE,
		kind: 'script'
	}
}

function flowItemToDraft(item: WorkspaceItem): Flow {
	const draftValue = item.value as FlowDraftValue | undefined
	if (!draftValue?.value) {
		throw new Error(`Draft flow "${item.path}" is missing value.`)
	}
	const value = clone(draftValue.value)
	if (draftValue.groups !== undefined && draftValue.groups !== null) {
		value.groups = clone(draftValue.groups)
	}
	return {
		path: item.path,
		summary: item.summary ?? '',
		value,
		schema: draftValue.schema ?? emptySchema(),
		edited_by: '',
		edited_at: '',
		archived: false,
		extra_perms: {}
	}
}

function appItemToDraft(item: WorkspaceItem): AppDraftValue {
	const value = item.value as AppDraftValue | undefined
	if (!value?.files || !value?.runnables) {
		throw new Error(`Draft app "${item.path}" is missing files or runnables.`)
	}
	return normalizeAppDraftValue({
		...clone(value),
		summary: value.summary ?? item.summary
	})
}

function scheduleItemToDraft(item: WorkspaceItem): NewSchedule {
	const value = item.value as NewSchedule | undefined
	if (!value) throw new Error(`Draft schedule "${item.path}" is missing value.`)
	return { ...clone(value), path: item.path }
}

function triggerItemToDraft(item: WorkspaceItem): TriggerRequestBody {
	const value = item.value as TriggerRequestBody | undefined
	if (!item.triggerKind) throw new Error(`Draft trigger "${item.path}" is missing trigger kind.`)
	if (!value) throw new Error(`Draft trigger "${item.path}" is missing value.`)
	return { ...clone(value), path: item.path } as TriggerRequestBody
}

function resourceItemToDraft(item: WorkspaceItem): ResourceDraftState {
	const value = item.value as CreateResource | undefined
	if (!value) throw new Error(`Draft resource "${item.path}" is missing value.`)
	return {
		path: item.path,
		description: value.description ?? '',
		args: clone((value.value ?? {}) as Record<string, any>),
		labels: value.labels,
		wsSpecific: value.ws_specific ?? false,
		resource_type: value.resource_type
	}
}

function variableItemToDraft(item: WorkspaceItem): VariableDraftState {
	const value = item.value as CreateVariable | undefined
	if (!value) throw new Error(`Draft variable "${item.path}" is missing value.`)
	return {
		path: item.path,
		variable: {
			value: value.value,
			is_secret: value.is_secret,
			description: value.description
		},
		labels: value.labels,
		wsSpecific: value.ws_specific ?? false,
		account: value.account,
		is_oauth: value.is_oauth,
		expires_at: value.expires_at
	}
}

function itemToDraftValue(item: WorkspaceItem): unknown {
	switch (item.type) {
		case 'script':
			return scriptItemToDraft(item)
		case 'flow':
			return flowItemToDraft(item)
		case 'app':
			return appItemToDraft(item)
		case 'schedule':
			return scheduleItemToDraft(item)
		case 'trigger':
			return triggerItemToDraft(item)
		case 'resource':
			return resourceItemToDraft(item)
		case 'variable':
			return variableItemToDraft(item)
	}
}

function draftSlotPriority(entry: UserDraftEntry, path: string): number {
	const exactStoragePath = entry.path === path
	if (entry.live && exactStoragePath) return 0
	if (entry.live) return 1
	if (exactStoragePath) return 2
	return 3
}

export function getGlobalDraftSlot<T = unknown>(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): GlobalDraftSlot<T> | undefined {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined

	const candidates: Array<{ entry: UserDraftEntry; item: WorkspaceItem; priority: number }> = []
	for (const entry of UserDraft.list({ workspace, itemKinds: [itemKind] })) {
		const item = userDraftEntryToWorkspaceItem(entry)
		if (!item) continue
		if (item.type !== type) continue
		if (type === 'trigger' && item.triggerKind !== triggerKind) continue
		if (item.path !== path && entry.path !== path) continue
		candidates.push({ entry, item, priority: draftSlotPriority(entry, path) })
	}

	candidates.sort((a, b) => a.priority - b.priority)
	const best = candidates[0]
	if (!best || best.entry.value === undefined) return undefined

	return {
		itemKind,
		storagePath: best.entry.path,
		value: best.entry.value as T,
		item: best.item
	}
}

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	return getGlobalDraftSlot(workspace, type, path, triggerKind)?.item
}

export function listGlobalDrafts(workspace: string): WorkspaceItem[] {
	const drafts = new Map<string, { item: WorkspaceItem; priority: number }>()
	for (const entry of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		const draft = userDraftEntryToWorkspaceItem(entry)
		if (!draft) continue
		const key = getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind)
		const priority = draftSlotPriority(entry, draft.path)
		const existing = drafts.get(key)
		if (!existing || priority < existing.priority) {
			drafts.set(key, { item: draft, priority })
		}
	}
	return Array.from(drafts.values()).map(({ item }) => item)
}

export function saveGlobalDraftValue<T>(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	value: T,
	triggerKind?: TriggerKind
): WorkspaceItem {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) {
		throw new Error(`Draft ${type} "${path}" is missing a UserDraft kind.`)
	}
	const storagePath = getGlobalDraftSlot(workspace, type, path, triggerKind)?.storagePath ?? path
	UserDraft.save(itemKind, storagePath, value, { workspace })
	const stored = getGlobalDraft(workspace, type, path, triggerKind)
	if (!stored) {
		throw new Error(`Could not read written draft ${type} "${path}".`)
	}
	return stored
}

export function setGlobalDraft(workspace: string, item: WorkspaceItem): WorkspaceItem {
	return saveGlobalDraftValue(
		workspace,
		item.type,
		item.path,
		itemToDraftValue(item),
		item.triggerKind
	)
}

export function saveGlobalAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue
): WorkspaceItem {
	const storagePath = getGlobalDraftSlot(workspace, 'app', path)?.storagePath ?? path
	UserDraft.save('raw_app', storagePath, normalizeAppDraftValue(value), { workspace })
	const stored = getGlobalDraft(workspace, 'app', path)
	if (!stored) throw new Error(`Could not read written app draft "${path}".`)
	return stored
}

export function deleteGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): void {
	const slot = getGlobalDraftSlot(workspace, type, path, triggerKind)
	if (slot) {
		UserDraft.clear(slot.itemKind, slot.storagePath, { workspace })
		return
	}
	const itemKind = itemKindFor(type, triggerKind)
	if (itemKind) UserDraft.clear(itemKind, path, { workspace })
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		UserDraft.clear(draft.itemKind, draft.path, { workspace })
	}
}
