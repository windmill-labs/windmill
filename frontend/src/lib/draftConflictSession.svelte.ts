/**
 * Identity for one editing session's conflict resolution.
 *
 * A drawer editor outlives what it opens: reopening the same item reuses the component with the
 * same workspace and path, so an awaited step cannot tell from those alone whether it still
 * speaks for what is on screen. Every resolution takes a token and checks it before touching
 * shared editor state.
 */
export function useDraftConflictSession() {
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
