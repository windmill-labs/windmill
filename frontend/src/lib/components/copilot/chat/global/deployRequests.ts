import type { Flow, NewScript, OpenFlowWPath, Script } from '$lib/gen/types.gen'
import type { FlowDraftValue, WorkspaceItem } from './workspaceItems'

type ScriptWithDeployMetadata = Script & Partial<Pick<NewScript, 'assets' | 'cache_ignore_s3_path'>>

export type FlowDeployRequestBody = OpenFlowWPath & {
	deployment_message?: string
}

function preserveOnBehalfOf(email: string | undefined): true | undefined {
	return email ? true : undefined
}

export function buildScriptDeployRequestBody(
	path: string,
	draft: WorkspaceItem,
	existing: Script | undefined,
	deploymentMessage: string | undefined
): NewScript {
	if (typeof draft.value !== 'string' || !draft.language) {
		throw new Error(`Draft script "${path}" is missing content or language.`)
	}

	const existingWithMetadata = existing as ScriptWithDeployMetadata | undefined

	return {
		path,
		summary: draft.summary ?? existing?.summary ?? '',
		description: existing?.description ?? '',
		content: draft.value,
		parent_hash: existing?.hash,
		schema: existing?.schema,
		is_template: existing?.is_template,
		language: draft.language,
		kind: existing?.kind,
		tag: existing?.tag,
		envs: existing?.envs,
		concurrent_limit: existing?.concurrent_limit,
		concurrency_time_window_s: existing?.concurrency_time_window_s,
		debounce_key: existing?.debounce_key,
		debounce_delay_s: existing?.debounce_delay_s,
		debounce_args_to_accumulate: existing?.debounce_args_to_accumulate,
		max_total_debouncing_time: existing?.max_total_debouncing_time,
		max_total_debounces_amount: existing?.max_total_debounces_amount,
		cache_ttl: existing?.cache_ttl,
		cache_ignore_s3_path: existingWithMetadata?.cache_ignore_s3_path,
		dedicated_worker: existing?.dedicated_worker,
		ws_error_handler_muted: existing?.ws_error_handler_muted,
		priority: existing?.priority,
		restart_unless_cancelled: existing?.restart_unless_cancelled,
		timeout: existing?.timeout,
		delete_after_secs: existing?.delete_after_secs,
		deployment_message: deploymentMessage,
		concurrency_key: existing?.concurrency_key,
		visible_to_runner_only: existing?.visible_to_runner_only,
		auto_kind: existing?.auto_kind,
		codebase: existing?.codebase,
		has_preprocessor: existing?.has_preprocessor,
		on_behalf_of_email: existing?.on_behalf_of_email,
		preserve_on_behalf_of: preserveOnBehalfOf(existing?.on_behalf_of_email),
		assets: existingWithMetadata?.assets,
		modules: existing?.modules,
		labels: existing?.labels
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
	deploymentMessage: string | undefined
): FlowDeployRequestBody {
	return {
		path,
		summary: draftSummary ?? existing?.summary ?? '',
		description: existing?.description ?? '',
		value: flowValueWithDraftGroups(flowDraft),
		schema: flowDraft.schema ?? existing?.schema ?? {},
		tag: existing?.tag,
		ws_error_handler_muted: existing?.ws_error_handler_muted,
		priority: existing?.priority,
		dedicated_worker: existing?.dedicated_worker,
		timeout: existing?.timeout,
		visible_to_runner_only: existing?.visible_to_runner_only,
		on_behalf_of_email: existing?.on_behalf_of_email,
		preserve_on_behalf_of: preserveOnBehalfOf(existing?.on_behalf_of_email),
		labels: existing?.labels,
		deployment_message: deploymentMessage
	}
}
