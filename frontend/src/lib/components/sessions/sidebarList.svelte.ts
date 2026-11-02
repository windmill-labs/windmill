import { BROWSER } from 'esm-env'
import { fromStore } from 'svelte/store'
import { roleCapabilities } from '$lib/stores'
import { copilotInfo } from '$lib/aiStore'
import {
	clearSessionsBetaOptOut,
	isGlobalAiEnabled,
	sessionsAllowedFor
} from '$lib/components/copilot/chat/global/gate'

const capabilities = fromStore(roleCapabilities)
const copilot = fromStore(copilotInfo)
// Listing sessions from within a session's preview frame would nest the whole experience.
const embedded = BROWSER && window.self !== window.top

/** Whether the sidebar lists AI sessions. Not in a browser that opted out of them, for a role
 * /sessions refuses, in a workspace that hid the assistant, nor inside a preview frame. */
export function sidebarSessionsShown(): boolean {
	return (
		!embedded &&
		isGlobalAiEnabled() &&
		sessionsAllowedFor(!capabilities.current.code) &&
		!copilot.current.workspaceDisabled
	)
}

/** Turning AI Sessions back on: Home, with the sessions listed in the sidebar. */
export function activateAiSessions(): void {
	clearSessionsBetaOptOut('/')
}
