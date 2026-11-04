import { BROWSER } from 'esm-env'
import { fromStore } from 'svelte/store'
import { copilotInfo } from '$lib/aiStore'

const copilot = fromStore(copilotInfo)
// Listing sessions from within a session's preview frame would nest the whole experience.
const embedded = BROWSER && window.self !== window.top

/** Whether the sidebar lists AI sessions. Not in a workspace that hid the assistant, nor inside a
 * preview frame. */
export function sidebarSessionsShown(): boolean {
	return !embedded && !copilot.current.workspaceDisabled
}
