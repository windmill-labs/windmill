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
	ScriptLang
} from '$lib/gen/types.gen'

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

export type WorkspaceItemType = 'script' | 'flow' | 'schedule' | 'trigger' | 'resource' | 'variable'

export type WorkspaceItem = {
	type: WorkspaceItemType
	path: string
	summary?: string
	language?: ScriptLang
	triggerKind?: TriggerKind
	value?:
		| string
		| FlowValue
		| NewSchedule
		| TriggerRequestBody
		| CreateResource
		| CreateVariable
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
	private drafts = $state<Record<string, WorkspaceItem>>({})

	listDrafts(): WorkspaceItem[] {
		return Object.values(this.drafts).map(clone)
	}

	getDraft(
		type: WorkspaceItemType,
		path: string,
		triggerKind?: TriggerKind
	): WorkspaceItem | undefined {
		const draft = this.drafts[getWorkspaceItemKey(type, path, triggerKind)]
		return draft ? clone(draft) : undefined
	}

	setDraft(item: WorkspaceItem): WorkspaceItem {
		const stored: WorkspaceItem = { ...clone(item), isDraft: true }
		this.drafts[getWorkspaceItemKey(item.type, item.path, item.triggerKind)] = stored
		return clone(stored)
	}

	deleteDraft(type: WorkspaceItemType, path: string, triggerKind?: TriggerKind): void {
		delete this.drafts[getWorkspaceItemKey(type, path, triggerKind)]
	}

	clearDrafts(): void {
		this.drafts = {}
	}

	getScriptDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('script', path)
	}

	getFlowDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('flow', path)
	}

	getScheduleDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('schedule', path)
	}

	getTriggerDraft(kind: TriggerKind, path: string): WorkspaceItem | undefined {
		return this.getDraft('trigger', path, kind)
	}

	getResourceDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('resource', path)
	}

	getVariableDraft(path: string): WorkspaceItem | undefined {
		return this.getDraft('variable', path)
	}
}

export const globalDraftStore = new GlobalDraftStore()
