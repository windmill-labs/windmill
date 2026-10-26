import { FlowService, ScriptService } from '$lib/gen'
import { resource } from 'runed'
import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'

type DeployedItem = {
	ws_error_handler_muted?: boolean
	on_behalf_of?: string
	on_behalf_of_email?: string
}

/** The deployed script or flow at `path`, read from its row: an editor's loaded item is the
 *  deployed one with its draft laid over it, so its mute flag and run-as user are the draft's,
 *  which can differ from what is deployed. `path` is undefined while nothing is deployed, and
 *  `current` is undefined then, while loading, and when the lookup fails. */
export function useDeployedItem(
	kind: 'script' | 'flow',
	workspaceGetter: () => string | undefined,
	pathGetter: () => string | undefined
) {
	const item = resource(
		() => ({ workspace: workspaceGetter(), path: pathGetter() }),
		async ({ workspace, path }): Promise<DeployedItem | undefined> => {
			if (!workspace || !path) return undefined
			try {
				return kind === 'flow'
					? await FlowService.getFlowByPath({ workspace, path })
					: await ScriptService.getScriptByPath({ workspace, path })
			} catch {
				return undefined
			}
		}
	)

	return {
		get current() {
			return pathGetter() ? item.current : undefined
		},
		get muted() {
			return !!item.current?.ws_error_handler_muted
		},
		/** Reads the row again, for a deploy to compare against what is deployed right now. */
		async refetch() {
			return pathGetter() ? await item.refetch() : undefined
		},
		async toggle() {
			const path = pathGetter()
			if (!path) return
			const next = await toggleWorkspaceErrorHandler(
				workspaceGetter(),
				kind,
				path,
				item.current?.ws_error_handler_muted
			)
			if (next !== undefined && item.current) {
				item.mutate({ ...item.current, ws_error_handler_muted: next })
			}
		}
	}
}
