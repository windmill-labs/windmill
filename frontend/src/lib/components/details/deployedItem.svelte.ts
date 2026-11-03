import { FlowService, ScriptService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
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
			// A click asks for the opposite of what the switch shows. Without the row the switch
			// shows "off" whatever is deployed, and would never show a muted item as muted: read
			// the row first, and leave a flag already at the asked value as it is.
			const muted = !item.current?.ws_error_handler_muted
			const deployed = item.current ?? (await item.refetch())
			if (!deployed) {
				sendUserToast(`Could not read the deployed ${kind}, try again`, true)
				return
			}
			if (!!deployed.ws_error_handler_muted === muted) return
			const next = await toggleWorkspaceErrorHandler(workspaceGetter(), kind, path, !muted)
			if (next !== undefined) {
				item.mutate({ ...deployed, ws_error_handler_muted: next })
			}
		}
	}
}
