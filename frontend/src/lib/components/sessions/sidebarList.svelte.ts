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

/** The list the sidebar shows while Home is open. Every other page decides its own (a
 * session's page lists sessions, any other page the workspace nav) and, on arrival, sets
 * this to it: Home sits above the switch and belongs to both, so it keeps the list you
 * came from until the switch changes it. Per browser, so a reload of Home keeps it too. */
export const homeSidebarList = useLocalStorageValue<SidebarList>(
	'windmill_sidebar_list',
	'workspace',
	'string'
)

/** The list the sidebar shows beside `pathname`, where it offers sessions. */
export function sidebarListFor(pathname: string): SidebarList {
	if (pathname.startsWith('/sessions')) return 'sessions'
	return pathname === '/' ? homeSidebarList.val : 'workspace'
}

/** Turning AI Sessions back on asks to see them: Home, with the session list beside it. */
export function activateAiSessions(): void {
	homeSidebarList.val = 'sessions'
	clearSessionsBetaOptOut('/')
}
