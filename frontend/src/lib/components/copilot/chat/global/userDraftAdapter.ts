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

function resourceDraftToWorkspaceItem(path: string, draft: CreateResource): WorkspaceItem {
	return {
		type: 'resource',
		path,
		summary: draft.description ?? undefined,
		value: clone(draft),
		isDraft: true
	}
}

function variableDraftToWorkspaceItem(path: string, draft: CreateVariable): WorkspaceItem {
	return {
		type: 'variable',
		path,
		summary: draft.description ?? undefined,
		value: clone(draft),
		isDraft: true
	}
}

function userDraftEntryToWorkspaceItem(entry: UserDraftEntry): WorkspaceItem | undefined {
	switch (entry.itemKind) {
		case 'script':
			return scriptDraftToWorkspaceItem(entry.path, entry.value as NewScript)
		case 'flow':
			return flowDraftToWorkspaceItem(entry.path, entry.value as Flow)
		case 'raw_app':
			return appDraftToWorkspaceItem(entry.path, entry.value as AppDraftValue)
		case 'trigger_schedule':
			return scheduleDraftToWorkspaceItem(entry.path, entry.value as NewSchedule)
		case 'resource':
			return resourceDraftToWorkspaceItem(entry.path, entry.value as CreateResource)
		case 'variable':
			return variableDraftToWorkspaceItem(entry.path, entry.value as CreateVariable)
		default: {
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[entry.itemKind]
			return triggerKind
				? triggerDraftToWorkspaceItem(triggerKind, entry.path, entry.value as TriggerRequestBody)
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

function resourceItemToDraft(item: WorkspaceItem): CreateResource {
	const value = item.value as CreateResource | undefined
	if (!value) throw new Error(`Draft resource "${item.path}" is missing value.`)
	return { ...clone(value), path: item.path }
}

function variableItemToDraft(item: WorkspaceItem): CreateVariable {
	const value = item.value as CreateVariable | undefined
	if (!value) throw new Error(`Draft variable "${item.path}" is missing value.`)
	return { ...clone(value), path: item.path }
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

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return undefined
	const draft = UserDraft.get(itemKind, path, { workspace })
	if (draft === undefined) return undefined

	const entry = { workspace, itemKind, path, value: draft, meta: {}, persisted: false, live: false }
	return userDraftEntryToWorkspaceItem(entry)
}

export function listGlobalDrafts(workspace: string): WorkspaceItem[] {
	const drafts = new Map<string, WorkspaceItem>()
	for (const entry of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		const draft = userDraftEntryToWorkspaceItem(entry)
		if (!draft) continue
		drafts.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), draft)
	}
	return Array.from(drafts.values())
}

export function setGlobalDraft(workspace: string, item: WorkspaceItem): WorkspaceItem {
	const itemKind = itemKindFor(item.type, item.triggerKind)
	if (!itemKind) {
		throw new Error(`Draft ${item.type} "${item.path}" is missing a UserDraft kind.`)
	}
	UserDraft.save(itemKind, item.path, itemToDraftValue(item), { workspace })
	const stored = getGlobalDraft(workspace, item.type, item.path, item.triggerKind)
	if (!stored) {
		throw new Error(`Could not read written draft ${item.type} "${item.path}".`)
	}
	return stored
}

export function saveGlobalAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue
): WorkspaceItem {
	UserDraft.save('raw_app', path, normalizeAppDraftValue(value), { workspace })
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
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return
	UserDraft.clear(itemKind, path, { workspace })
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		UserDraft.clear(draft.itemKind, draft.path, { workspace })
	}
}
