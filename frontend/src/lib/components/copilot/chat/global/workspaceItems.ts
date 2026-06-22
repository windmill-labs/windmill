import type {
	AzureTriggerData,
	CreateResource,
	CreateVariable,
	Flow,
	FlowValue,
	GcpTriggerData,
	NewHttpTrigger,
	NewKafkaTrigger,
	NewMqttTrigger,
	NewNatsTrigger,
	NewPostgresTrigger,
	NewSchedule,
	NewScript,
	NewSqsTrigger,
	NewWebsocketTrigger,
	Policy,
	Script,
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
	description?: string | null
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

/**
 * Editable metadata surfaced on read and accepted by the `set_*_metadata` tools.
 * A flat superset across scripts, flows, and apps; each producer fills only the
 * fields relevant to its type. Carried on `WorkspaceItem` so it survives the
 * lossy draft -> WorkspaceItem projection (which otherwise keeps only
 * summary/language/value). `summary` stays a top-level WorkspaceItem field, so
 * it is intentionally absent here.
 */
export type ItemMetadata = {
	description?: string
	labels?: string[]
	tag?: string
	priority?: number
	timeout?: number
	concurrent_limit?: number
	concurrency_time_window_s?: number
	cache_ttl?: number
	/** scripts only */
	kind?: 'script' | 'failure' | 'trigger' | 'command' | 'approval' | 'preprocessor'
	/** apps only */
	execution_mode?: 'viewer' | 'publisher' | 'anonymous'
	/** apps only */
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
	metadata?: ItemMetadata
	isDraft: boolean
	isLiveDraft?: boolean
}

/** Extract the editable metadata from a script (draft `NewScript` or deployed `Script`). */
export function scriptItemMetadata(s: NewScript | Script): ItemMetadata {
	return {
		description: s.description,
		labels: s.labels,
		kind: s.kind,
		tag: s.tag,
		priority: s.priority,
		timeout: s.timeout,
		concurrent_limit: s.concurrent_limit,
		concurrency_time_window_s: s.concurrency_time_window_s,
		cache_ttl: s.cache_ttl
	}
}

/**
 * Extract the editable metadata from a flow. Summary/description/tag/timeout/labels
 * live on the `Flow` wrapper; priority/concurrency/cache_ttl live inside `value`.
 */
export function flowItemMetadata(f: Flow): ItemMetadata {
	const v = f.value as FlowValue | undefined
	return {
		description: f.description,
		labels: f.labels,
		tag: f.tag,
		timeout: f.timeout,
		priority: v?.priority,
		concurrent_limit: v?.concurrent_limit,
		concurrency_time_window_s: v?.concurrency_time_window_s,
		cache_ttl: v?.cache_ttl
	}
}

/** Extract the editable metadata from a raw app draft value. */
export function appItemMetadata(v: AppDraftValue): ItemMetadata {
	return {
		execution_mode: v.policy?.execution_mode,
		custom_path: v.custom_path
	}
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
