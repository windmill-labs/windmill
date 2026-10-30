import { fromStore, get } from 'svelte/store'
import { userWorkspaces, workspaceStore } from '$lib/stores'
import { switchWorkspace } from '$lib/storeUtils'
import { useEditRights } from '$lib/operatorWriteRights'
import { buildsAnything } from '$lib/editRights'
import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
import { workspaceAIClients } from '$lib/components/copilot/lib'
import { fixupUrlAfterWorkspaceSwitch } from '$lib/components/sidebar/workspaceSwitchUrl'
import type { WorkModeLock } from '../copilot/chat/workMode'
import { useSessionReadOnly } from './sessionPermissions.svelte'
import {
	getEffectiveWorkspaceId,
	setSessionPendingWorkspace,
	type Session
} from './sessionState.svelte'
import { openNewSession } from './sessionSwitch.svelte'

// Home's draft acts on the workspace the header names, so there the header moves with the
// session, the way the header's own picker switches it.
function moveUnsentSession(sessionId: string, id: string, switchesWorkspace: boolean) {
	setSessionPendingWorkspace(sessionId, id)
	if (!switchesWorkspace || get(workspaceStore) === id) return
	workspaceAIClients.init(id)
	switchWorkspace(id)
	void fixupUrlAfterWorkspaceSwitch(id)
}

/**
 * What holds a session's chat in Use, for its mode picker: a protected workspace, whose way
 * out is its dev workspace, or a role that builds nothing there. `switchesWorkspace` is for
 * Home: moving its draft to the dev workspace moves the page too. Reads context and registers
 * an effect: call during component initialisation.
 */
export function useWorkModeLock(session: () => Session | undefined, switchesWorkspace = false) {
	const workspace = () => {
		const s = session()
		return s ? getEffectiveWorkspaceId(s) : undefined
	}
	const protectedWorkspace = useSessionReadOnly(workspace)
	const rights = useEditRights(workspace)
	const workspaces = fromStore(userWorkspaces)
	const lock = $derived.by((): WorkModeLock | undefined => {
		const s = session()
		const ws = workspace()
		// A staged fork is created at the first send and the session builds there, so nothing
		// about the workspace it forks holds the session in Use.
		if (!s || !ws || s.pending_fork) return undefined
		if (protectedWorkspace.known) {
			const dev = findCanonicalDevWorkspace(ws, workspaces.current)
			if (!dev)
				return { reason: 'This workspace is protected against direct changes. Build in a fork.' }
			const label = `${dev.name} (dev)`
			return {
				reason: `This workspace is protected against direct changes. Build in ${label}.`,
				// Until its first message a session can still change workspace; after it, only a new
				// one can act elsewhere: Home's composer, on the dev workspace.
				escape: s.workspace_id
					? {
							label: `New session in ${label}`,
							action: () => void openNewSession(dev.id)
						}
					: {
							label: `Switch to ${label}`,
							action: () => moveUnsentSession(s.id, dev.id, switchesWorkspace)
						}
			}
		}
		if (!buildsAnything(rights.capabilities)) {
			return {
				reason: "Your role can't build in this workspace. A workspace admin can change that."
			}
		}
		return undefined
	})
	return {
		get current() {
			return lock
		}
	}
}
