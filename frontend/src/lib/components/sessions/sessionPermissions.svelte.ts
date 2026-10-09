import { fromStore } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import { useActingUser } from '$lib/actingUser.svelte'
import { capabilitiesForRole } from '../copilot/chat/global/sessionAccess'
import { canOpenEditor, type EditorKind } from '../copilot/chat/sessionCapabilities'
import { previewPageAllowed, type PreviewPage } from './previewRouter'

/**
 * What the session UI may offer in `workspace`, from the same `capabilitiesForRole` that
 * narrows the chat's tools, so a control and the tool behind it never disagree.
 *
 * Unknown user answers yes, as the toolset does before access resolves: hiding every editor
 * from a developer while `whoami` is in flight is the worse mistake, and the editor itself
 * refuses an operator. Reads context and registers an effect: call during component
 * initialisation.
 */
export function useSessionPermissions(workspace: () => string | undefined) {
	const user = useActingUser(workspace)
	const workspaces = fromStore(userWorkspaces)
	const entry = $derived(workspaces.current.find((w) => w.id === workspace()))
	const access = $derived.by(() => {
		const u = user.current
		if (!u) return undefined
		return capabilitiesForRole({
			isAdmin: u.is_admin || u.is_super_admin,
			operator: u.operator,
			// Unread here: nothing below asks for `deploy`.
			deployRulesPass: true,
			operatorSettings: entry?.operator_settings
		})
	})

	return {
		/** Whether this user can save what the `kind` editor writes. */
		canOpenEditor: (kind: EditorKind) => canOpenEditor(access, kind),
		/** Whether the page loads for this user rather than refusing them. */
		pageAllowed: (page: PreviewPage) => previewPageAllowed(page, user.current, entry)
	}
}

/** The editor whose draft rules apply to an item kind. Any app, raw or not, saves app drafts. */
export function editorKindOf(kind: string): EditorKind | undefined {
	if (kind === 'script' || kind === 'flow' || kind === 'pipeline') return kind
	if (kind === 'raw_app' || kind === 'app') return 'raw_app'
	return undefined
}
