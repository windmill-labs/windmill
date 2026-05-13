import type {
	AzureTriggerData,
	CreateResource,
	CreateVariable,
	FlowValue,
	GcpTriggerData,
	NewHttpTrigger,
	NewKafkaTrigger,
	NewMqttTrigger,
	NewNatsTrigger,
	NewPostgresTrigger,
	NewSchedule,
	NewSqsTrigger,
	NewWebsocketTrigger,
	Policy,
	ScriptLang
} from '$lib/gen/types.gen'

/**
 * Flow draft value. Mirrors what the backend's create/update flow API expects
 * — the OpenFlow value, plus the inputs schema and (optional) groups.
 *
 * Schema and groups are split out from FlowValue intentionally so that
 * deploy_workspace_item can preserve them through the draft → workspace
 * round-trip; an earlier version dropped them on every deploy.
 */
export type FlowDraftValue = {
	value: FlowValue
	schema?: Record<string, any> | null
	groups?: NonNullable<FlowValue['groups']> | null
}

export const TRIGGER_KINDS = [
	'http',
	'websocket',
	'kafka',
	'nats',
	'postgres',
	'mqtt',
	'sqs',
	'gcp',
	'azure'
] as const

export type TriggerKind = (typeof TRIGGER_KINDS)[number]

export type TriggerRequestBody =
	| NewHttpTrigger
	| NewWebsocketTrigger
	| NewKafkaTrigger
	| NewNatsTrigger
	| NewPostgresTrigger
	| NewMqttTrigger
	| NewSqsTrigger
	| GcpTriggerData
	| AzureTriggerData

export type WorkspaceItemType =
	| 'script'
	| 'flow'
	| 'schedule'
	| 'trigger'
	| 'resource'
	| 'variable'
	| 'app'

export type AppDraftValue = {
	summary?: string
	files: Record<string, string>
	runnables: Record<string, any>
	data?: any
	policy?: Policy
	custom_path?: string
}

export type WorkspaceItem = {
	type: WorkspaceItemType
	path: string
	summary?: string
	language?: ScriptLang
	triggerKind?: TriggerKind
	value?:
		| string
		| FlowDraftValue
		| NewSchedule
		| TriggerRequestBody
		| CreateResource
		| CreateVariable
		| AppDraftValue
	isDraft: boolean
}

export function getWorkspaceItemKey(
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): string {
	if (type === 'trigger') {
		return `trigger:${triggerKind ?? ''}:${path}`
	}
	return `${type}:${path}`
}

function clone<T>(value: T): T {
	return structuredClone($state.snapshot(value)) as T
}

class GlobalDraftStore {
	private drafts = $state<Record<string, Record<string, WorkspaceItem>>>({})

	private getWorkspaceDrafts(workspace: string): Record<string, WorkspaceItem> {
		return this.drafts[workspace] ?? {}
	}

	private ensureWorkspaceDrafts(workspace: string): Record<string, WorkspaceItem> {
		if (!this.drafts[workspace]) {
			this.drafts[workspace] = {}
		}
		return this.drafts[workspace]
	}

	listDrafts(workspace: string): WorkspaceItem[] {
		return Object.values(this.getWorkspaceDrafts(workspace)).map(clone)
	}

	getDraft(
		workspace: string,
		type: WorkspaceItemType,
		path: string,
		triggerKind?: TriggerKind
	): WorkspaceItem | undefined {
		const draft = this.getWorkspaceDrafts(workspace)[getWorkspaceItemKey(type, path, triggerKind)]
		return draft ? clone(draft) : undefined
	}

	setDraft(workspace: string, item: WorkspaceItem): WorkspaceItem {
		const stored: WorkspaceItem = { ...clone(item), isDraft: true }
		this.ensureWorkspaceDrafts(workspace)[
			getWorkspaceItemKey(item.type, item.path, item.triggerKind)
		] = stored
		return clone(stored)
	}

	deleteDraft(
		workspace: string,
		type: WorkspaceItemType,
		path: string,
		triggerKind?: TriggerKind
	): void {
		const drafts = this.drafts[workspace]
		if (!drafts) return

		delete drafts[getWorkspaceItemKey(type, path, triggerKind)]
		if (Object.keys(drafts).length === 0) {
			delete this.drafts[workspace]
		}
	}

	clearDrafts(workspace: string): void {
		delete this.drafts[workspace]
	}

	getScriptDraft(workspace: string, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'script', path)
	}

	getFlowDraft(workspace: string, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'flow', path)
	}

	getScheduleDraft(workspace: string, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'schedule', path)
	}

	getTriggerDraft(workspace: string, kind: TriggerKind, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'trigger', path, kind)
	}

	getResourceDraft(workspace: string, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'resource', path)
	}

	getVariableDraft(workspace: string, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'variable', path)
	}

	getAppDraft(workspace: string, path: string): WorkspaceItem | undefined {
		return this.getDraft(workspace, 'app', path)
	}
}

export const globalDraftStore = new GlobalDraftStore()
