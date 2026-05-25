import type { Flow, NewSchedule, NewScript } from '$lib/gen/types.gen'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import {
	UserDraft,
	type UserDraftEntry,
	type UserDraftItemKind,
	type UserDraftMeta
} from '$lib/userDraft.svelte'
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

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	return getGlobalDraftSlot(workspace, type, path, triggerKind)?.item
}

export function listGlobalDrafts(workspace: string): WorkspaceItem[] {
	const drafts = new Map<string, WorkspaceItem>()
	for (const entry of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		const { displayPath, isLiveDraft } = liveDisplayPath(workspace, entry.itemKind, entry.path)
		const draft = userDraftEntryToWorkspaceItem(entry, displayPath, isLiveDraft)
		if (!draft) continue
		drafts.set(getWorkspaceItemKey(draft.type, draft.path, draft.triggerKind), draft)
	}
	return Array.from(drafts.values())
}

export function saveGlobalAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue,
	meta?: UserDraftMeta
): WorkspaceItem {
	const storagePath = resolveDraftStoragePath(workspace, 'raw_app', path)
	const normalized = normalizeAppDraftValue(value)
	if (meta) {
		UserDraft.setDraftAndMeta('raw_app', storagePath, normalized, meta, { workspace })
	} else {
		UserDraft.save('raw_app', storagePath, normalized, { workspace })
	}
	const stored = getGlobalDraft(workspace, 'app', path)
	if (!stored) throw new Error(`Could not read written app draft "${path}".`)
	return stored
}

type DeleteGlobalDraftOptions = {
	preserveLiveDraft?: boolean
}

export function deleteGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind,
	options: DeleteGlobalDraftOptions = {}
): void {
	const itemKind = itemKindFor(type, triggerKind)
	if (!itemKind) return
	const storagePath = resolveDraftStoragePath(workspace, itemKind, path)
	const liveDraft = UserDraft.getLiveEditorDraft(itemKind, { workspace })
	if (options.preserveLiveDraft && liveDraft?.storagePath === storagePath) {
		UserDraft.remove(itemKind, storagePath, { workspace })
	} else {
		UserDraft.clear(itemKind, storagePath, { workspace })
	}
	if (type === 'variable') clearEphemeralSecretVariableDraftValue(workspace, storagePath)
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...GLOBAL_DRAFT_KINDS] })) {
		UserDraft.clear(draft.itemKind, draft.path, { workspace })
	}
	clearEphemeralSecretVariableDraftValues(workspace)
}
