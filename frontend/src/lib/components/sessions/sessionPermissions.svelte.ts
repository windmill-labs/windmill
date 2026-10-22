import { getContext } from 'svelte'
import { fromStore } from 'svelte/store'
import { resource } from 'runed'
import { userWorkspaces } from '$lib/stores'
import { useActingUser } from '$lib/actingUser.svelte'
import { useEditRights } from '$lib/operatorWriteRights'
import { buildsAnything } from '$lib/roleCapabilities'
import { isReadOnlySessionWorkspace } from '$lib/components/copilot/chat/global/sessionAccess'
import { AIWorkMode } from '$lib/components/copilot/chat/workMode'
import type { WorkspaceItem } from '$lib/components/workspacePicker'
import { previewPageAllowed, type PreviewPage } from './previewRouter'

/**
 * Whether a session in `workspace` is read-only: the workspace is protected by
 * `DisableDirectDeployment`, which holds every user, admins included. The same verdict
 * `resolveSessionAccess` gives the chat, so the UI and the toolset agree. Reads as false until
 * resolved, and on a failed check.
 */
export function useSessionReadOnly(workspace: () => string | undefined) {
	const verdict = resource(workspace, async (ws) => (ws ? isReadOnlySessionWorkspace(ws) : false))
	return {
		get current() {
			return !!workspace() && verdict.current === true
		}
	}
}

/** The session chat whose work mode the session UI follows. */
type WorkModeHost = { readonly workMode: AIWorkMode }

/**
 * What the session UI may offer in `workspace`. Its editors save drafts, so it reads
 * `roleCanDraft` / `canDraftItem` — the rule the chat's draft tools and `open_preview` follow — so
 * a control and the tool behind it never disagree. An unknown user reads as able to draft, as the toolset does before access
 * resolves: hiding every editor from a developer while `whoami` is in flight is the worse
 * mistake, and the server refuses an operator anyway. A session held in Use opens no editor: one
 * in a protected workspace, one whose role builds nothing there, or one whose chat is in Use
 * mode — the chat `chat` returns, else the `aiChatManager` context's.
 * Reads context and registers an effect: call during component initialisation.
 */
export function useSessionPermissions(
	workspace: () => string | undefined,
	chat?: () => WorkModeHost | undefined
) {
	const contextChat = getContext<WorkModeHost | undefined>('aiChatManager')
	const rights = useEditRights(workspace)
	const user = useActingUser(workspace)
	const protectedWorkspace = useSessionReadOnly(workspace)
	const readOnly = $derived(
		protectedWorkspace.current ||
			!buildsAnything(rights.capabilities) ||
			(chat?.() ?? contextChat)?.workMode === AIWorkMode.USE
	)
	const workspaces = fromStore(userWorkspaces)
	const entry = $derived(workspaces.current.find((w) => w.id === workspace()))

	return {
		/** The session is read-only in this workspace. */
		get readOnly() {
			return readOnly
		},
		/** Whether this user can save the drafts the `kind` editor writes. */
		canOpenEditor: (kind: string) => !readOnly && rights.roleCanDraft(kind),
		/** The same, and this item's own permissions let them write it. */
		canEditItem: (item: Pick<WorkspaceItem, 'kind' | 'raw_app' | 'path' | 'extraPerms'>) =>
			!readOnly &&
			rights.canDraftItem(item.raw_app ? 'raw_app' : item.kind, item.path, item.extraPerms),
		/** Whether the page loads for this user rather than refusing them. */
		pageAllowed: (page: PreviewPage) =>
			// A read-only session reaches what an operator reaches: no admin page.
			!(readOnly && page.adminOnly) && previewPageAllowed(page, user.current, entry)
	}
}
