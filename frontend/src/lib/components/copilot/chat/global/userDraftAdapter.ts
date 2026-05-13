import type { Flow, NewSchedule, NewScript } from '$lib/gen/types.gen'
import { UserDraft, type UserDraftItemKind, type UserDraftListEntry } from '$lib/userDraft.svelte'
import {
	getWorkspaceItemKey,
	globalDraftStore,
	type AppDraftValue,
	type TriggerRequestBody,
	type TriggerKind,
	type WorkspaceItem,
	type WorkspaceItemType
} from './draftStore.svelte'

type SharedWorkspaceItemType = 'script' | 'flow' | 'app' | 'schedule' | 'trigger'

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
	'trigger_azure'
] as const satisfies UserDraftItemKind[]
const DEFAULT_APP_DATA = { tables: [], datatable: undefined, schema: undefined }

function clone<T>(value: T): T {
	return structuredClone(value) as T
}

function isSharedWorkspaceItemType(type: WorkspaceItemType): type is SharedWorkspaceItemType {
	return (
		type === 'script' ||
		type === 'flow' ||
		type === 'app' ||
		type === 'schedule' ||
		type === 'trigger'
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
	}
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

function sharedDraftEntryToWorkspaceItem(entry: UserDraftListEntry): WorkspaceItem | undefined {
	switch (entry.itemKind) {
		case 'script':
			return scriptDraftToWorkspaceItem(entry.path, entry.value as NewScript)
		case 'flow':
			return flowDraftToWorkspaceItem(entry.path, entry.value as Flow)
		case 'raw_app':
			return appDraftToWorkspaceItem(entry.path, entry.value as AppDraftValue)
		case 'trigger_schedule':
			return scheduleDraftToWorkspaceItem(entry.path, entry.value as NewSchedule)
		default:
			const triggerKind = TRIGGER_KIND_BY_DRAFT_KIND[entry.itemKind]
			return triggerKind
				? triggerDraftToWorkspaceItem(triggerKind, entry.path, entry.value as TriggerRequestBody)
				: undefined
	}
}

function getSharedDraft(
	workspace: string,
	type: SharedWorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	const itemKind = sharedDraftKind(type, triggerKind)
	if (!itemKind) return undefined
	const draft = UserDraft.get(itemKind, path, { workspace })
	if (draft === undefined) return undefined

	switch (type) {
		case 'script':
			return scriptDraftToWorkspaceItem(path, draft as NewScript)
		case 'flow':
			return flowDraftToWorkspaceItem(path, draft as Flow)
		case 'app':
			return appDraftToWorkspaceItem(path, draft as AppDraftValue)
		case 'schedule':
			return scheduleDraftToWorkspaceItem(path, draft as NewSchedule)
		case 'trigger':
			return triggerKind
				? triggerDraftToWorkspaceItem(triggerKind, path, draft as TriggerRequestBody)
				: undefined
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
	UserDraft.remove(itemKind, path, { workspace })
}

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	if (isSharedWorkspaceItemType(type)) {
		const shared = getSharedDraft(workspace, type, path, triggerKind)
		if (shared) return shared
	}
	return globalDraftStore.getDraft(workspace, type, path, triggerKind)
}

export function listGlobalDrafts(workspace: string): WorkspaceItem[] {
	const drafts = new Map<string, WorkspaceItem>()

	for (const draft of globalDraftStore.listDrafts(workspace)) {
		drafts.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), draft)
	}

	for (const entry of UserDraft.list({ workspace, itemKinds: [...SHARED_DRAFT_KINDS] })) {
		const draft = sharedDraftEntryToWorkspaceItem(entry)
		if (!draft) continue
		drafts.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), draft)
	}

	return Array.from(drafts.values())
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
	globalDraftStore.deleteDraft(workspace, type, path, triggerKind)
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...SHARED_DRAFT_KINDS] })) {
		UserDraft.remove(draft.itemKind, draft.path, { workspace })
	}
	globalDraftStore.clearDrafts(workspace)
}

export function triggerKindToUserDraftKind(kind: TriggerKind): UserDraftItemKind {
	return TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[kind]
}
