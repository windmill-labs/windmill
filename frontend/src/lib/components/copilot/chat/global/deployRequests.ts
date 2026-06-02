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

function deployMetadataField<T extends object, K extends keyof T>(
	draftMetadata: T | undefined,
	existing: T | undefined,
	key: K
): T[K] | undefined {
	if (draftMetadata && Object.prototype.hasOwnProperty.call(draftMetadata, key)) {
		return draftMetadata[key]
	}
	return existing?.[key]
}

export function buildScriptDeployRequestBody(
	path: string,
	draft: WorkspaceItem,
	existing: Script | undefined,
	deploymentMessage: string | undefined,
	draftMetadata?: ScriptDeployMetadata
): NewScript {
	if (typeof draft.value !== 'string' || !draft.language) {
		throw new Error(`Draft script "${path}" is missing content or language.`)
	}

	const existingWithMetadata = existing as ScriptDeployMetadata | undefined
	const onBehalfOfEmail = deployMetadataField(
		draftMetadata,
		existingWithMetadata,
		'on_behalf_of_email'
	)

	return {
		path,
		summary:
			draft.summary ?? deployMetadataField(draftMetadata, existingWithMetadata, 'summary') ?? '',
		description: deployMetadataField(draftMetadata, existingWithMetadata, 'description') ?? '',
		content: draft.value,
		parent_hash: existing?.hash,
		schema: deployMetadataField(draftMetadata, existingWithMetadata, 'schema'),
		is_template: deployMetadataField(draftMetadata, existingWithMetadata, 'is_template'),
		language: draft.language,
		kind: deployMetadataField(draftMetadata, existingWithMetadata, 'kind'),
		tag: deployMetadataField(draftMetadata, existingWithMetadata, 'tag'),
		envs: deployMetadataField(draftMetadata, existingWithMetadata, 'envs'),
		concurrent_limit: deployMetadataField(draftMetadata, existingWithMetadata, 'concurrent_limit'),
		concurrency_time_window_s: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'concurrency_time_window_s'
		),
		debounce_key: deployMetadataField(draftMetadata, existingWithMetadata, 'debounce_key'),
		debounce_delay_s: deployMetadataField(draftMetadata, existingWithMetadata, 'debounce_delay_s'),
		debounce_args_to_accumulate: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'debounce_args_to_accumulate'
		),
		max_total_debouncing_time: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'max_total_debouncing_time'
		),
		max_total_debounces_amount: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'max_total_debounces_amount'
		),
		cache_ttl: deployMetadataField(draftMetadata, existingWithMetadata, 'cache_ttl'),
		cache_ignore_s3_path: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'cache_ignore_s3_path'
		),
		dedicated_worker: deployMetadataField(draftMetadata, existingWithMetadata, 'dedicated_worker'),
		ws_error_handler_muted: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'ws_error_handler_muted'
		),
		priority: deployMetadataField(draftMetadata, existingWithMetadata, 'priority'),
		restart_unless_cancelled: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'restart_unless_cancelled'
		),
		timeout: deployMetadataField(draftMetadata, existingWithMetadata, 'timeout'),
		delete_after_secs: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'delete_after_secs'
		),
		deployment_message: deploymentMessage,
		concurrency_key: deployMetadataField(draftMetadata, existingWithMetadata, 'concurrency_key'),
		visible_to_runner_only: deployMetadataField(
			draftMetadata,
			existingWithMetadata,
			'visible_to_runner_only'
		),
		auto_kind: deployMetadataField(draftMetadata, existingWithMetadata, 'auto_kind'),
		codebase: deployMetadataField(draftMetadata, existingWithMetadata, 'codebase'),
		has_preprocessor: deployMetadataField(draftMetadata, existingWithMetadata, 'has_preprocessor'),
		on_behalf_of_email: onBehalfOfEmail,
		preserve_on_behalf_of: preserveOnBehalfOf(onBehalfOfEmail),
		assets: deployMetadataField(draftMetadata, existingWithMetadata, 'assets'),
		modules: deployMetadataField(draftMetadata, existingWithMetadata, 'modules'),
		labels: deployMetadataField(draftMetadata, existingWithMetadata, 'labels')
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
	existing: Flow | undefined,
	deploymentMessage: string | undefined,
	draftMetadata?: FlowDeployMetadata
): FlowDeployRequestBody {
	const onBehalfOfEmail = deployMetadataField(draftMetadata, existing, 'on_behalf_of_email')

	return {
		path,
		summary: draftSummary ?? deployMetadataField(draftMetadata, existing, 'summary') ?? '',
		description: deployMetadataField(draftMetadata, existing, 'description') ?? '',
		value: flowValueWithDraftGroups(flowDraft),
		schema: flowDraft.schema ?? deployMetadataField(draftMetadata, existing, 'schema') ?? {},
		tag: deployMetadataField(draftMetadata, existing, 'tag'),
		ws_error_handler_muted: deployMetadataField(draftMetadata, existing, 'ws_error_handler_muted'),
		priority: deployMetadataField(draftMetadata, existing, 'priority'),
		dedicated_worker: deployMetadataField(draftMetadata, existing, 'dedicated_worker'),
		timeout: deployMetadataField(draftMetadata, existing, 'timeout'),
		visible_to_runner_only: deployMetadataField(draftMetadata, existing, 'visible_to_runner_only'),
		on_behalf_of_email: onBehalfOfEmail,
		preserve_on_behalf_of: preserveOnBehalfOf(onBehalfOfEmail),
		labels: deployMetadataField(draftMetadata, existing, 'labels'),
		deployment_message: deploymentMessage
	}
}
