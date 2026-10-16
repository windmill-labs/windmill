import { scopedKey } from '$lib/userScopedStorage'

const WELCOMED_KEY = 'session-block-welcomed-workspaces'
/** A membership older than this is not a first visit, whatever this browser remembers: without
 * it, everyone already in a workspace would be welcomed to it once on their next visit. */
const NEW_MEMBER_MS = 30 * 24 * 60 * 60 * 1000

// Workspaces welcomed during this page load: the welcome holds for the whole visit (Home, the
// empty session, back), and only a later load greets by time of day.
const welcomedThisLoad = new Set<string>()

/** Whether this is the user's first visit to `workspace`: a recent member this browser has not
 * welcomed there yet. Answering yes records the welcome. */
export function takeFirstVisit(workspace: string, memberSince: string | undefined): boolean {
	if (welcomedThisLoad.has(workspace)) return true
	const key = scopedKey(WELCOMED_KEY)
	if (!key) return false
	let welcomed: string[] = []
	try {
		welcomed = JSON.parse(localStorage.getItem(key) ?? '[]')
	} catch {}
	if (welcomed.includes(workspace)) return false
	const joined = memberSince ? Date.parse(memberSince) : NaN
	if (!(Date.now() - joined < NEW_MEMBER_MS)) return false
	welcomedThisLoad.add(workspace)
	try {
		localStorage.setItem(key, JSON.stringify([...welcomed, workspace]))
	} catch {}
	return true
}

export function timeOfDayGreeting(now = new Date()): string {
	const hour = now.getHours()
	if (hour >= 5 && hour < 12) return 'Good morning'
	if (hour >= 12 && hour < 18) return 'Good afternoon'
	return 'Good evening'
}
