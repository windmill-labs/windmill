import { resource } from 'runed'
import { FolderService } from '$lib/gen'
import type { AIChatManager } from './AIChatManager.svelte'
import { scopeOptions, type ScopeOption } from './workingScope'

/** The roots the chat's picker offers, read live from its manager, for the workspace the
 * session acts on. */
export function useWorkingInOptions(
	manager: AIChatManager,
	workspace: () => string | undefined
): { readonly options: ScopeOption[] } {
	const folders = resource(workspace, async (ws) => {
		if (!ws) return []
		// The send path loads these; a picker opened before the first send needs them for the
		// user's own row and the badges. Not mid-turn: a refresh would invalidate the send's
		// own, as AssistantSettingsModal's refresh explains.
		if (!manager.loading && !manager.sendInFlight) {
			void manager.refreshGlobalIdentity()
			void manager.refreshGlobalFolderInstructions()
		}
		const names = await FolderService.listFolderNames({ workspace: ws })
		// "Acting on" can move an unsent session to a workspace without some chosen folders:
		// they drop out, and with none left the session works in the whole workspace.
		const roots = manager.workingIn
		const kept = roots?.filter((r) => !r.startsWith('f/') || names.includes(r.slice(2)))
		if (roots && kept && kept.length < roots.length && manager.messages.length === 0) {
			if (ws === workspace()) manager.setWorkingIn(kept.length ? kept : undefined, { fallback: true })
		}
		return names
	})
	return {
		get options() {
			return scopeOptions(folders.current ?? [], manager.scopeIdentity, manager.scopeResources)
		}
	}
}
