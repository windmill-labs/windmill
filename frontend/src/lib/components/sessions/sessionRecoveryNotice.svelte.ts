import { SvelteSet } from 'svelte/reactivity'

// Session ids opened as a stand-in for a `session_name` this browser doesn't
// hold. Kept in memory rather than on the Session record: persisted, the notice
// would replay on every reload of a session the user has since made their own.
const recovered = new SvelteSet<string>()

export function markSessionRecovered(id: string): void {
	recovered.add(id)
}

export function isSessionRecovered(id: string): boolean {
	return recovered.has(id)
}

export function clearSessionRecovered(id: string): void {
	recovered.delete(id)
}
