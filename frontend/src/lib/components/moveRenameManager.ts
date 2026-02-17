import { AppService, FlowService, ScriptService } from '$lib/gen'

type ItemKind = 'flow' | 'script' | 'app'

/**
 * Check whether a flow uses on_behalf_of_email.
 * Callers use this to show a warning before saving.
 */
export async function checkFlowOnBehalfOf(
	workspace: string,
	path: string
): Promise<string | undefined> {
	const flow = await FlowService.getFlowByPath({ workspace, path })
	return flow.on_behalf_of_email
}

/**
 * Shared utility that performs the API call to update an item's path and summary.
 * No toast, no navigation — callers handle those.
 *
 * Note: on_behalf_of_email is intentionally omitted from flow updates for security
 * reasons — the backend will redeploy the flow on behalf of the current user.
 */
export async function updateItemPathAndSummary(opts: {
	workspace: string
	kind: ItemKind
	initialPath: string
	newPath: string
	newSummary: string
}): Promise<void> {
	const { workspace, kind, initialPath, newPath, newSummary } = opts

	if (kind === 'flow') {
		const flow = await FlowService.getFlowByPath({ workspace, path: initialPath })
		await FlowService.updateFlow({
			workspace,
			path: initialPath,
			requestBody: {
				path: newPath,
				summary: newSummary,
				description: flow.description,
				value: flow.value,
				schema: flow.schema,
				tag: flow.tag,
				dedicated_worker: flow.dedicated_worker,
				ws_error_handler_muted: flow.ws_error_handler_muted,
				visible_to_runner_only: flow.visible_to_runner_only
			}
		})
	} else if (kind === 'script') {
		const script = await ScriptService.getScriptByPath({ workspace, path: initialPath })
		script.summary = newSummary
		await ScriptService.createScript({
			workspace,
			requestBody: {
				...script,
				description: script.description ?? '',
				lock: script.lock,
				parent_hash: script.hash,
				path: newPath
			}
		})
	} else if (kind === 'app') {
		await AppService.updateApp({
			workspace,
			path: initialPath,
			requestBody: {
				path: newPath !== initialPath ? newPath : undefined,
				summary: newSummary
			}
		})
	}
}
