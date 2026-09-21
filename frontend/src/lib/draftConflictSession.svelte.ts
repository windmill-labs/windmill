import { untrack } from 'svelte'

/**
 * Identity for one editing session's conflict resolution.
 *
 * An editor outlives what it shows, so an awaited step cannot tell from the workspace and path
 * alone whether it still speaks for what is on screen: both come back to the values it started
 * with when the user returns to where they were. Every resolution takes a token and checks it
 * before touching shared editor state.
 */
export function createDraftConflictSession() {
	let generation = $state(0)
	/** The token of the resolution in flight, or 0. A token rather than a flag: one left over from
	 *  a closed session must not clear the busy state of the session that replaced it. */
	let inFlight = $state(0)

	return {
		/** A resolution started in the current session is in flight. Ending a session clears it, so
		 *  a reopened editor never shows buttons that only a stale request settling can re-enable. */
		get busy(): boolean {
			return inFlight !== 0
		},
		/** Claim the session for a resolution about to start. */
		start(): number {
			inFlight = ++generation
			return inFlight
		},
		/** Whether `token` still speaks for the editor. */
		holds(token: number): boolean {
			return token === generation
		},
		/** Release `token`'s claim. A stale token is ignored. */
		finish(token: number): void {
			if (inFlight === token) inFlight = 0
		},
		/** Nothing outstanding speaks for this editor any more: a different item, a different
		 *  session on the same one, or the component going away. */
		end(): void {
			generation++
			inFlight = 0
		}
	}
}

/**
 * A session for an editor that swaps what it shows in place, `scopeOf` naming what that currently
 * is. Leaving a scope and coming back is a new session, so the change itself ends the old one:
 * afterwards every value a resolution captured before the switch reads as current again, and no
 * later comparison could tell it apart from one that never left.
 */
export function useDraftConflictSession(scopeOf: () => unknown) {
	const session = createDraftConflictSession()

	let lastScope = untrack(scopeOf)
	$effect(() => {
		const next = scopeOf()
		untrack(() => {
			if (next === lastScope) return
			lastScope = next
			session.end()
		})
	})

	return session
}
