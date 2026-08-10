import type {
	AzureTriggerData,
	CreateResource,
	CreateVariable,
	FlowValue,
	GcpTriggerData,
	NewEmailTrigger,
	NewHttpTrigger,
	NewKafkaTrigger,
	NewMqttTrigger,
	NewAmqpTrigger,
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
	description?: string | null
}

export const TRIGGER_KINDS = [
	'http',
	'websocket',
	'kafka',
	'nats',
	'postgres',
	'mqtt',
	'amqp',
	'sqs',
	'gcp',
	'azure',
	'email'
] as const

export type TriggerKind = (typeof TRIGGER_KINDS)[number]

export type TriggerRequestBody =
	| NewHttpTrigger
	| NewWebsocketTrigger
	| NewKafkaTrigger
	| NewNatsTrigger
	| NewPostgresTrigger
	| NewMqttTrigger
	| NewAmqpTrigger
	| NewSqsTrigger
	| GcpTriggerData
	| AzureTriggerData
	| NewEmailTrigger

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
	// Fork base: the deployed app version this draft was started from, pinned at
	// fork. The app analog of a script's parent_hash / a flow's version_id.
	parent_version?: number
	// User-typed friendly path while the app is parked at a `…/draft_<uuid>`
	// storage path (see RawAppDraft in sessions/appDraftCodec.ts). Must
	// round-trip through chat writes or an edit erases the chosen name.
	draft_path?: string
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
	/** THE invariant for secret variable drafts: `variable.value` is `''` when nothing
	 * is staged, so any non-empty value is a staged one — either an `$encrypted:`
	 * marker the draft endpoint encrypted at rest, or plaintext still only in the
	 * in-tab cell. A value the chat itself wrote is redacted to `''` and kept in
	 * memory instead, which is what this flag exists for: it records that a rotation
	 * IS staged, so a lost in-memory value is distinguishable from a metadata-only
	 * edit and cannot deploy as a rotation that never happened. Sticky until the
	 * draft is deployed or discarded. */
	pendingSecretValue?: boolean
}

export type WorkspaceItem = {
	type: WorkspaceItemType
	path: string
	/** Friendly display path for a draft parked at a `…/draft_<uuid>` storage
	 * path (the draft value's `draft_path`). Display-only — `path` is the key
	 * drafts are stored and routed under. */
	draftPath?: string
	summary?: string
	language?: ScriptLang
	triggerKind?: TriggerKind
	// Fork base: the deployed version this draft was started from, compared against
	// the current deployed head to detect a stale draft. `parentHash` for scripts
	// (script hash), `parentVersionId` for flows (flow_version id).
	parentHash?: string
	parentVersionId?: number
	value?:
		| string
		| FlowDraftValue
		| NewSchedule
		| TriggerRequestBody
		| CreateResource
		| CreateVariable
		| AppDraftValue
	/** Input schema of a script read (flows carry theirs inside `value`). */
	schema?: unknown
	/** Variables only. The one piece of a secret variable that is safe to report:
	 * without it a reader cannot tell a secret from a plain variable, since the
	 * value is always redacted. */
	isSecret?: boolean
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
