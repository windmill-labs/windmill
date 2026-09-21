import { get } from 'svelte/store'
import { FlowService, ScriptService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { workspaceStore } from '$lib/stores'

/** Flips the workspace error handler for one script or flow and reports the outcome in a
 * toast. Returns the new muted state, or undefined when nothing changed. */
export async function toggleWorkspaceErrorHandler(
	kind: 'script' | 'flow',
	path: string,
	muted: boolean | undefined
): Promise<boolean | undefined> {
	const workspace = get(workspaceStore)
	if (workspace === undefined) return undefined
	const next = !muted
	try {
		if (kind === 'flow') {
			await FlowService.toggleWorkspaceErrorHandlerForFlow({
				workspace,
				path,
				requestBody: { muted: next }
			})
		} else {
			await ScriptService.toggleWorkspaceErrorHandlerForScript({
				workspace,
				path,
				requestBody: { muted: next }
			})
		}
	} catch (error) {
		sendUserToast(
			`Error while toggling Workspace Error Handler: ${error.body || error.message}`,
			true
		)
		return undefined
	}
	sendUserToast(next ? 'Workspace error handler muted' : 'Workspace error handler active', false)
	return next
}
