import { BROWSER } from 'esm-env'
import { fromStore } from 'svelte/store'
import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
import { roleCapabilities } from '$lib/stores'
import { copilotInfo } from '$lib/aiStore'
import {
	clearSessionsBetaOptOut,
	isGlobalAiEnabled,
	sessionsAllowedFor
} from '$lib/components/copilot/chat/global/gate'

export type SidebarList = 'workspace' | 'sessions'

const capabilities = fromStore(roleCapabilities)
const copilot = fromStore(copilotInfo)
// Listing sessions from within a session's preview frame would nest the whole experience.
const embedded = BROWSER && window.self !== window.top

/** Whether the sidebar offers AI sessions: the Workspace ⇄ AI Sessions switch and the session
 * list. Not in a browser that opted out of them, for a role /sessions refuses, in a workspace
 * that hid the assistant, nor inside a preview frame. */
export function sessionsSwitchShown(): boolean {
	return (
		!embedded &&
		isGlobalAiEnabled() &&
		sessionsAllowedFor(!capabilities.current.code) &&
		!copilot.current.workspaceDisabled
	)
}

/** The list the sidebar shows, on every page. Only the Workspace ⇄ AI Sessions switch
 * changes it: navigating never does, so a session's page can sit beside the workspace nav
 * and a workspace page beside the session list. Per browser, so a reload keeps it. */
export const chosenSidebarList = useLocalStorageValue<SidebarList>(
	'windmill_sidebar_list',
	'workspace',
	'string'
)

/** Turning AI Sessions back on asks to see them: Home, with the session list beside it. */
export function activateAiSessions(): void {
	chosenSidebarList.val = 'sessions'
	clearSessionsBetaOptOut('/')
}
