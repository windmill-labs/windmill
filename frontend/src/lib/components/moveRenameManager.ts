import { AppService, FlowService, ResourceService, ScriptService } from '$lib/gen'
import { agentEditorRefusal } from './flows/agentResourceUtils'
import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'

type ItemKind = 'flow' | 'script' | 'app' | 'agent'

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
 *
 * `skip_draft_deletion` on every call: this re-deploys the DEPLOYED content at a
 * new path, so the caller's draft is unrelated work, not the thing being
 * deployed. Without the flag the backend would delete it. The backend carries
 * every remaining draft at the old path over to the new one.
 */
export async function updateItemPathAndSummary(opts: {
	workspace: string
	kind: ItemKind
	initialPath: string
	newPath: string
	newSummary: string
	labels?: string[]
}): Promise<void> {
	const { workspace, kind, initialPath, newPath, newSummary, labels } = opts

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
				visible_to_runner_only: flow.visible_to_runner_only,
				labels,
				skip_draft_deletion: true
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
				path: newPath,
				labels,
				skip_draft_deletion: true
			}
		})
	} else if (kind === 'agent') {
		// An agent is a resource, and its summary is the resource's description. The read before
		// the write is the same one the agent editor does: an update carries no resource type, so
		// a path deleted and recreated as something else in the meantime would otherwise take an
		// agent's config.
		const current = await ResourceService.getResource({ workspace, path: initialPath })
		const refused = agentEditorRefusal(initialPath, current.resource_type)
		if (refused) {
			throw new Error(refused)
		}
		// Only what this popover edits: the backend sets a column per field it is given, so the
		// agent's own config, its type and `ws_specific` are left where they are rather than read
		// here and written back over whatever changed in between.
		await ResourceService.updateResource({
			workspace,
			path: initialPath,
			requestBody: { path: newPath, description: newSummary, labels }
		})
	} else if (kind === 'app') {
		await AppService.updateApp({
			workspace,
			path: initialPath,
			requestBody: {
				path: newPath !== initialPath ? newPath : undefined,
				summary: newSummary,
				labels,
				skip_draft_deletion: true
			}
		})
	}

	// The path changed (rename/move) — drop the autocomplete cache so the new
	// path shows up immediately instead of after the 60s TTL.
	invalidateWorkspacePaths(workspace)
}
