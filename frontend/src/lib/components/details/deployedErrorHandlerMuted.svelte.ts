import { FlowService, ScriptService } from '$lib/gen'
import { resource } from 'runed'
import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'

/** Whether the deployed script or flow at `path` has the workspace error handler muted, read from
 *  its row: an editor's loaded item carries its draft's copy of the flag, which can be stale.
 *  `path` is undefined while nothing is deployed. */
export function useDeployedErrorHandlerMuted(
	kind: 'script' | 'flow',
	workspaceGetter: () => string | undefined,
	pathGetter: () => string | undefined
) {
	const muted = resource(
		() => ({ workspace: workspaceGetter(), path: pathGetter() }),
		async ({ workspace, path }) => {
			if (!workspace || !path) return false
			try {
				const item =
					kind === 'flow'
						? await FlowService.getFlowByPath({ workspace, path })
						: await ScriptService.getScriptByPath({ workspace, path })
				return !!item.ws_error_handler_muted
			} catch {
				return false
			}
		}
	)

	return {
		get muted() {
			return muted.current ?? false
		},
		async toggle() {
			const path = pathGetter()
			if (!path) return
			const next = await toggleWorkspaceErrorHandler(workspaceGetter(), kind, path, muted.current)
			if (next !== undefined) muted.mutate(next)
		}
	}
}
