import { FlowService, ScriptService } from '$lib/gen'
import type { Flow, NewSchedule, NewScript, Script, ScriptLang } from '$lib/gen/types.gen'
import { UserDraft, type UserDraftItemKind, type UserDraftListEntry } from '$lib/userDraft.svelte'
import { emptySchema } from '$lib/utils'
import {
	getWorkspaceItemKey,
	globalDraftStore,
	type AppDraftValue,
	type FlowDraftValue,
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
const DEFAULT_SCRIPT_LANGUAGE: ScriptLang = 'bun'
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

function applyItemSummary<T extends object>(value: T, summary: string | undefined): T {
	const draft = value as T & { summary?: string | null }
	if (draft.summary === undefined && summary !== undefined) {
		draft.summary = summary
	}
	return value
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

function scriptItemToUserDraft(item: WorkspaceItem, existing?: Script): NewScript {
	if (typeof item.value !== 'string') {
		throw new Error(`Draft script "${item.path}" is missing source content.`)
	}

	if (existing) {
		return {
			...clone(existing),
			parent_hash: existing.hash,
			path: item.path,
			summary: item.summary ?? existing.summary,
			content: item.value,
			language: item.language ?? existing.language
		}
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

function flowItemToUserDraft(item: WorkspaceItem, existing?: Flow): Flow {
	const draftValue = item.value as FlowDraftValue | undefined
	if (!draftValue?.value) {
		throw new Error(`Draft flow "${item.path}" is missing value.`)
	}

	const value = clone(draftValue.value)
	if (draftValue.groups !== undefined && draftValue.groups !== null) {
		value.groups = clone(draftValue.groups)
	}

	if (existing) {
		return {
			...clone(existing),
			path: item.path,
			summary: item.summary ?? existing.summary,
			value,
			schema: draftValue.schema ?? existing.schema
		}
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

function appItemToUserDraft(item: WorkspaceItem): AppDraftValue {
	const value = item.value as AppDraftValue | undefined
	if (!value?.files || !value?.runnables) {
		throw new Error(`Draft app "${item.path}" is missing files or runnables.`)
	}
	return normalizeAppDraftValue({
		...clone(value),
		summary: value.summary ?? item.summary
	})
}

function scheduleItemToUserDraft(item: WorkspaceItem): NewSchedule {
	const value = item.value as NewSchedule | undefined
	if (!value) {
		throw new Error(`Draft schedule "${item.path}" is missing value.`)
	}
	const draft = {
		...clone(value),
		path: item.path
	}
	return applyItemSummary(draft, item.summary)
}

function triggerItemToUserDraft(item: WorkspaceItem): TriggerRequestBody {
	const value = item.value as TriggerRequestBody | undefined
	if (!item.triggerKind) {
		throw new Error(`Draft trigger "${item.path}" is missing trigger kind.`)
	}
	if (!value) {
		throw new Error(`Draft trigger "${item.path}" is missing value.`)
	}
	const draft = {
		...clone(value),
		path: item.path
	}
	return applyItemSummary(draft, item.summary)
}

async function loadExistingScript(
	workspace: string,
	path: string,
	loadExisting: boolean | undefined
): Promise<Script | undefined> {
	if (!loadExisting) return undefined
	return ScriptService.getScriptByPath({ workspace, path })
}

async function loadExistingFlow(
	workspace: string,
	path: string,
	loadExisting: boolean | undefined
): Promise<Flow | undefined> {
	if (!loadExisting) return undefined
	return FlowService.getFlowByPath({ workspace, path })
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

async function setSharedDraft(
	workspace: string,
	item: WorkspaceItem,
	loadExisting: boolean | undefined
): Promise<WorkspaceItem> {
	switch (item.type) {
		case 'script': {
			const draft = scriptItemToUserDraft(
				item,
				await loadExistingScript(workspace, item.path, loadExisting)
			)
			UserDraft.save('script', item.path, draft, { workspace })
			return scriptDraftToWorkspaceItem(item.path, draft)
		}
		case 'flow': {
			const draft = flowItemToUserDraft(
				item,
				await loadExistingFlow(workspace, item.path, loadExisting)
			)
			UserDraft.save('flow', item.path, draft, { workspace })
			return flowDraftToWorkspaceItem(item.path, draft)
		}
		case 'app': {
			const draft = appItemToUserDraft(item)
			UserDraft.save('raw_app', item.path, draft, { workspace })
			return appDraftToWorkspaceItem(item.path, draft)
		}
		case 'schedule': {
			const draft = scheduleItemToUserDraft(item)
			UserDraft.save('trigger_schedule', item.path, draft, { workspace })
			return scheduleDraftToWorkspaceItem(item.path, draft)
		}
		case 'trigger': {
			if (!item.triggerKind) {
				throw new Error(`Draft trigger "${item.path}" is missing trigger kind.`)
			}
			const draft = triggerItemToUserDraft(item)
			UserDraft.save(TRIGGER_DRAFT_KIND_BY_TRIGGER_KIND[item.triggerKind], item.path, draft, {
				workspace
			})
			return triggerDraftToWorkspaceItem(item.triggerKind, item.path, draft)
		}
		default:
			throw new Error(`Unsupported shared draft type: ${item.type}`)
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

export async function setGlobalDraft(
	workspace: string,
	item: WorkspaceItem,
	opts?: { loadExisting?: boolean }
): Promise<WorkspaceItem> {
	if (isSharedWorkspaceItemType(item.type)) {
		return setSharedDraft(workspace, item, opts?.loadExisting)
	}
	return globalDraftStore.setDraft(workspace, item)
}

export function saveGlobalAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue
): WorkspaceItem {
	const draft = appDraftToWorkspaceItem(path, value)
	UserDraft.save('raw_app', path, draft.value as AppDraftValue, { workspace })
	return draft
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
