import type { Flow, NewScript, OpenFlowWPath, Script } from '$lib/gen/types.gen'
import type { FlowDraftValue, WorkspaceItem } from './workspaceItems'

export type ScriptDeployMetadata = Partial<Script> & Partial<NewScript>
export type FlowDeployMetadata = Partial<Flow>

export type FlowDeployRequestBody = OpenFlowWPath & {
	deployment_message?: string
}

function preserveOnBehalfOf(email: string | undefined): true | undefined {
	return email ? true : undefined
}

const SCRIPT_DEPLOY_METADATA_FIELDS = [
	'schema',
	'is_template',
	'kind',
	'tag',
	'envs',
	'concurrent_limit',
	'concurrency_time_window_s',
	'debounce_key',
	'debounce_delay_s',
	'debounce_args_to_accumulate',
	'max_total_debouncing_time',
	'max_total_debounces_amount',
	'cache_ttl',
	'cache_ignore_s3_path',
	'dedicated_worker',
	'ws_error_handler_muted',
	'priority',
	'restart_unless_cancelled',
	'timeout',
	'delete_after_secs',
	'concurrency_key',
	'visible_to_runner_only',
	'auto_kind',
	'codebase',
	'has_preprocessor',
	'on_behalf_of_email',
	'assets',
	'modules',
	'labels'
] as const satisfies readonly (keyof ScriptDeployMetadata)[]

const FLOW_DEPLOY_METADATA_FIELDS = [
	'tag',
	'ws_error_handler_muted',
	'priority',
	'dedicated_worker',
	'timeout',
	'visible_to_runner_only',
	'on_behalf_of_email',
	'labels'
] as const satisfies readonly (keyof FlowDeployMetadata)[]

function mergeDeployMetadata<T extends object>(
	existing: T | undefined,
	draftMetadata: T | undefined
): T {
	return { ...existing, ...draftMetadata } as T
}

function pickDeployMetadata<T extends object, K extends keyof T>(
	source: T,
	fields: readonly K[]
): Pick<T, K> {
	return Object.fromEntries(fields.map((field) => [field, source[field]])) as Pick<T, K>
}

export function buildScriptDeployRequestBody(
	path: string,
	draft: WorkspaceItem,
	existing: ScriptDeployMetadata | undefined,
	deploymentMessage: string | undefined,
	draftMetadata?: ScriptDeployMetadata
): NewScript {
	if (typeof draft.value !== 'string' || !draft.language) {
		throw new Error(`Draft script "${path}" is missing content or language.`)
	}

	const metadata = mergeDeployMetadata(existing, draftMetadata)
	const onBehalfOfEmail = metadata.on_behalf_of_email

	return {
		path,
		summary: draft.summary ?? metadata.summary ?? '',
		description: metadata.description ?? '',
		content: draft.value,
		parent_hash: existing?.hash,
		...pickDeployMetadata(metadata, SCRIPT_DEPLOY_METADATA_FIELDS),
		language: draft.language,
		deployment_message: deploymentMessage,
		preserve_on_behalf_of: preserveOnBehalfOf(onBehalfOfEmail)
	}
}

function flowValueWithDraftGroups(flowDraft: FlowDraftValue): FlowDraftValue['value'] {
	if (flowDraft.groups === undefined) {
		return flowDraft.value
	}
	return {
		...flowDraft.value,
		groups: flowDraft.groups ?? undefined
	}
}

export function buildFlowDeployRequestBody(
	path: string,
	draftSummary: string | undefined,
	flowDraft: FlowDraftValue,
	existing: FlowDeployMetadata | undefined,
	deploymentMessage: string | undefined,
	draftMetadata?: FlowDeployMetadata
): FlowDeployRequestBody {
	const metadata = mergeDeployMetadata(existing, draftMetadata)
	const onBehalfOfEmail = metadata.on_behalf_of_email

	return {
		path,
		summary: draftSummary ?? metadata.summary ?? '',
		description: metadata.description ?? '',
		value: flowValueWithDraftGroups(flowDraft),
		schema: flowDraft.schema ?? metadata.schema ?? {},
		...pickDeployMetadata(metadata, FLOW_DEPLOY_METADATA_FIELDS),
		preserve_on_behalf_of: preserveOnBehalfOf(onBehalfOfEmail),
		deployment_message: deploymentMessage
	}
}
