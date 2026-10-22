import { fromStore } from 'svelte/store'
import { userWorkspaces } from '$lib/stores'
import { useEditRights } from '$lib/operatorWriteRights'
import { buildsAnything } from '$lib/roleCapabilities'
import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
import type { WorkModeLock } from '../copilot/chat/workMode'
import { pickSessionWorkspace } from './SessionWorkspaceBar.svelte'
import { useSessionReadOnly } from './sessionPermissions.svelte'
import { getEffectiveWorkspaceId, type Session } from './sessionState.svelte'
import { startSessionInWorkspace } from './sessionSwitch.svelte'

/**
 * What holds a session's chat in Use, for its mode picker: a protected workspace, whose way
 * out is its dev workspace, or a role that builds nothing there. `switchesWorkspace` is the
 * session's workspace bar's: on Home, moving the session to the dev workspace moves the page
 * too. Reads context and registers an effect: call during component initialisation.
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
		if (!s || !ws) return undefined
		if (protectedWorkspace.current) {
			const dev = findCanonicalDevWorkspace(ws, workspaces.current)
			if (!dev)
				return { reason: 'This workspace is protected against direct changes. Build in a fork.' }
			const label = `${dev.name} (dev)`
			return {
				reason: `This workspace is protected against direct changes. Build in ${label}.`,
				// Until its first message a session can still change workspace; after it, only a new
				// one can act elsewhere.
				escape: s.workspace_id
					? {
							label: `New session in ${label}`,
							action: () => void startSessionInWorkspace(dev.id)
						}
					: {
							label: `Switch to ${label}`,
							action: () => pickSessionWorkspace(s.id, dev.id, switchesWorkspace)
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
