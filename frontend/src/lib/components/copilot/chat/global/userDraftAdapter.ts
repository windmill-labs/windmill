import { FlowService, ScriptService } from '$lib/gen'
import type { Flow, NewScript, Script, ScriptLang } from '$lib/gen/types.gen'
import { UserDraft, type UserDraftItemKind, type UserDraftListEntry } from '$lib/userDraft.svelte'
import { emptySchema } from '$lib/utils'
import {
	getWorkspaceItemKey,
	globalDraftStore,
	type AppDraftValue,
	type FlowDraftValue,
	type TriggerKind,
	type WorkspaceItem,
	type WorkspaceItemType
} from './draftStore.svelte'

type SharedWorkspaceItemType = 'script' | 'flow' | 'app'

const SHARED_DRAFT_KINDS = ['script', 'flow', 'raw_app'] as const satisfies UserDraftItemKind[]
const DEFAULT_SCRIPT_LANGUAGE: ScriptLang = 'bun'
const DEFAULT_APP_DATA = { tables: [], datatable: undefined, schema: undefined }

function clone<T>(value: T): T {
	return structuredClone(value) as T
}

function isSharedWorkspaceItemType(type: WorkspaceItemType): type is SharedWorkspaceItemType {
	return type === 'script' || type === 'flow' || type === 'app'
}

function sharedDraftKind(type: SharedWorkspaceItemType): UserDraftItemKind {
	return type === 'app' ? 'raw_app' : type
}

function normalizeAppDraftValue(value: AppDraftValue): AppDraftValue {
	return {
		...value,
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: value.data ?? { ...DEFAULT_APP_DATA }
	}
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

function sharedDraftEntryToWorkspaceItem(entry: UserDraftListEntry): WorkspaceItem | undefined {
	switch (entry.itemKind) {
		case 'script':
			return scriptDraftToWorkspaceItem(entry.path, entry.value as NewScript)
		case 'flow':
			return flowDraftToWorkspaceItem(entry.path, entry.value as Flow)
		case 'raw_app':
			return appDraftToWorkspaceItem(entry.path, entry.value as AppDraftValue)
		default:
			return undefined
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
	path: string
): WorkspaceItem | undefined {
	const itemKind = sharedDraftKind(type)
	const draft = UserDraft.get(itemKind, path, { workspace })
	if (draft === undefined) return undefined

	switch (type) {
		case 'script':
			return scriptDraftToWorkspaceItem(path, draft as NewScript)
		case 'flow':
			return flowDraftToWorkspaceItem(path, draft as Flow)
		case 'app':
			return appDraftToWorkspaceItem(path, draft as AppDraftValue)
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
		default:
			throw new Error(`Unsupported shared draft type: ${item.type}`)
	}
}

function deleteSharedDraft(workspace: string, type: SharedWorkspaceItemType, path: string): void {
	UserDraft.remove(sharedDraftKind(type), path, { workspace })
}

export function getGlobalDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): WorkspaceItem | undefined {
	if (isSharedWorkspaceItemType(type)) {
		const shared = getSharedDraft(workspace, type, path)
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
		deleteSharedDraft(workspace, type, path)
	}
	globalDraftStore.deleteDraft(workspace, type, path, triggerKind)
}

export function clearGlobalDrafts(workspace: string): void {
	for (const draft of UserDraft.list({ workspace, itemKinds: [...SHARED_DRAFT_KINDS] })) {
		UserDraft.remove(draft.itemKind, draft.path, { workspace })
	}
	globalDraftStore.clearDrafts(workspace)
}
