import type { OnNavigate } from '@sveltejs/kit'

// Moving between Home and AI Sessions keeps the session block in sight: the browser
// snapshots the page before and after the navigation and glides each element that
// carries the same `view-transition-name` on both sides (AIChatDisplay names the
// heading and the composer); the rest of the page cross-fades. The timing lives in
// app.css, keyed on the `data-home-sessions-transition` attribute set here for the
// length of the move.

const HOME = '/'
const SESSIONS = '/sessions'

/** Whether the composer had focus when the user left AI Sessions for Home. */
let composerFocused = false

/** The focus flag to keep after a navigation from `from` to `to`. Only the return to Home
 * hands focus on — the sessions page focuses its composer on its own — and every other
 * navigation clears the flag, so a later arrival on Home never acts on a stale one. */
export function composerFocusAfter(
	from: string | undefined,
	to: string | undefined,
	composerHadFocus: boolean
): boolean {
	return from === SESSIONS && to === HOME && composerHadFocus
}

/** Whether Home should give its composer the focus the sessions page's had. Read once:
 * a later visit starts unfocused. */
export function takeComposerFocus(): boolean {
	const focused = composerFocused
	composerFocused = false
	return focused
}

/** For SvelteKit's `onNavigate`: runs a move between Home and AI Sessions as a view
 * transition. Any other navigation, or a browser without the API, is left alone. */
export function homeSessionsTransition(navigation: OnNavigate): Promise<void> | undefined {
	const from = navigation.from?.url.pathname
	const to = navigation.to?.url.pathname
	composerFocused = composerFocusAfter(
		from,
		to,
		document.activeElement?.closest('[data-session-composer]') != null
	)
	const direction =
		from === HOME && to === SESSIONS
			? 'enter'
			: from === SESSIONS && to === HOME
				? 'leave'
				: undefined
	if (!direction || navigation.willUnload || !document.startViewTransition) return
	const root = document.documentElement
	// Reduced motion keeps the transition but drops every name, so the two pages simply
	// cross-fade as one.
	root.dataset.homeSessionsTransition = window.matchMedia('(prefers-reduced-motion: reduce)')
		.matches
		? 'fade'
		: direction
	return new Promise((resolve) => {
		const transition = document.startViewTransition(async () => {
			resolve()
			await navigation.complete
		})
		// Rejects when the navigation is cancelled or fails; the attribute goes either way.
		const done = () => delete root.dataset.homeSessionsTransition
		void transition.finished.then(done, done)
	})
}
