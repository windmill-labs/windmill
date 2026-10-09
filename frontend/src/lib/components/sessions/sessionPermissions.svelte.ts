import { fromStore } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import { useActingUser } from '$lib/actingUser.svelte'
import { useEditRights } from '$lib/operatorWriteRights'
import type { WorkspaceItem } from '$lib/components/workspacePicker'
import { previewPageAllowed, type PreviewPage } from './previewRouter'

/**
 * What the session UI may offer in `workspace`, from the same `roleCanAuthor` / `canEditItem`
 * the rest of the app and the chat's toolset read, so a control and the tool behind it never
 * disagree. An unknown user's role reads as able to author, as the toolset does before access
 * resolves: hiding every editor from a developer while `whoami` is in flight is the worse
 * mistake, and the server refuses an operator anyway. Reads context and registers an effect:
 * call during component initialisation.
 */
export function useSessionPermissions(workspace: () => string | undefined) {
	const rights = useEditRights(workspace)
	const user = useActingUser(workspace)
	const workspaces = fromStore(userWorkspaces)
	const entry = $derived(workspaces.current.find((w) => w.id === workspace()))

	return {
		/** Whether this user can save the drafts the `kind` editor writes. */
		canOpenEditor: (kind: string) => rights.roleCanDraft(kind),
		/** The same, and this item's own permissions let them write it. */
		canEditItem: (item: Pick<WorkspaceItem, 'kind' | 'path' | 'extraPerms'>) =>
			rights.canDraftItem(item.kind, item.path, item.extraPerms),
		/** Whether the page loads for this user rather than refusing them. */
		pageAllowed: (page: PreviewPage) => previewPageAllowed(page, user.current, entry)
	}
}
