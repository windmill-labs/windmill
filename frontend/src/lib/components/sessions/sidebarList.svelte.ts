import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
import { clearSessionsBetaOptOut } from '$lib/components/copilot/chat/global/gate'

export type SidebarList = 'workspace' | 'sessions'

/** The list the sidebar shows while Home is open. Every other page decides its own (a
 * session's page lists sessions, any other page the workspace nav) and, on arrival, sets
 * this to it: Home sits above the switch and belongs to both, so it keeps the list you
 * came from, until you go into its composer (HomeAIChat). Per browser, so a reload of Home
 * keeps it too. */
export const homeSidebarList = useLocalStorageValue<SidebarList>(
	'windmill_sidebar_list',
	'workspace',
	'string'
)

/** The list the sidebar shows beside `pathname`. */
export function sidebarListFor(pathname: string): SidebarList {
	if (pathname.startsWith('/sessions')) return 'sessions'
	return pathname === '/' ? homeSidebarList.val : 'workspace'
}

/** Turning AI Sessions back on asks to see them: Home, with the session list beside it. */
export function activateAiSessions(): void {
	homeSidebarList.val = 'sessions'
	clearSessionsBetaOptOut('/')
}
