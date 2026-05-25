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
 * -- the OpenFlow value, plus the inputs schema and (optional) groups.
 *
 * Schema and groups are split out from FlowValue intentionally so that
 * deploy_workspace_item can preserve them through the draft -> workspace
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

export type ResourceDraftState = {
	path: string
	description: string
	args: Record<string, any>
	labels: string[] | undefined
	wsSpecific: boolean
	resource_type?: string
}

export type VariableDraftState = {
	path: string
	variable: { value: string; is_secret: boolean; description: string }
	labels: string[] | undefined
	wsSpecific: boolean
	account?: number
	is_oauth?: boolean
	expires_at?: string
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
	isLiveDraft?: boolean
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
