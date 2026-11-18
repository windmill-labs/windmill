import { FlowService, ScriptService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { Item } from '$lib/utils'
import { BellOff } from 'lucide-svelte'

/** The editor menu's switch for muting the item. Once it is deployed the switch writes to the
 * deployed version at once and no deploy carries it; before that, the first deploy does. The
 * caption says which, since the two look the same in the menu. */
export function muteErrorHandlerMenuItem(opts: {
	kind: 'script' | 'flow'
	deployed: boolean
	muted: boolean
	hasEnterpriseLicense: boolean
	onToggle: () => void
}): Item {
	return {
		displayName: 'Mute error handler',
		description: opts.deployed
			? `Applies right away: failures of the deployed ${opts.kind} stop triggering the workspace error handler`
			: `Applies when you deploy: failures of this ${opts.kind} won't trigger the workspace error handler`,
		icon: BellOff,
		toggle: opts.muted,
		// Without a license muting is not on offer, but a muted item can still be unmuted.
		disabled: !opts.hasEnterpriseLicense && !opts.muted,
		tooltip: opts.hasEnterpriseLicense ? undefined : 'Enterprise Edition only',
		separatorTop: true,
		action: () => opts.onToggle()
	}
}

/** Flips the workspace error handler for one script or flow and reports the outcome in a
 * toast. Returns the new muted state, or undefined when nothing changed. The workspace is the
 * caller's to supply — a detail page embedded in an AI session acts on that session's
 * workspace, which is not the one the navigation store names. */
export async function toggleWorkspaceErrorHandler(
	workspace: string | undefined,
	kind: 'script' | 'flow',
	path: string,
	muted: boolean | undefined
): Promise<boolean | undefined> {
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
