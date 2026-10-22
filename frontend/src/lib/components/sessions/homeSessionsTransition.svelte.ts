import type { OnNavigate } from '@sveltejs/kit'
import { sessionState } from './sessionState.svelte'

// Moving between Home and AI Sessions keeps the composer in sight: the browser
// snapshots the page before and after the navigation and glides the element that
// carries the `composer` view-transition-name on both sides (AIChatDisplay names it);
// the rest of the page cross-fades. The timing lives in app.css, keyed on the
// `data-home-sessions-transition` attribute set here for the length of the move.

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

// A session that has sent its first message has a composer of its own, not Home's draft, so
// a move between it and Home is not one composer moving: its page names no composer. The one
// exception is the send from Home itself, which lands on that session already under way.
let sentFromHome = $state(false)

/** Called by Home's composer as its send hands over to the session page. */
export function composerSentFromHome(): void {
	sentFromHome = true
}

/** Whether the session page arriving now came from a send on Home, so its composer is the
 * one Home just sent from. */
export function arrivingFromHomeSend(): boolean {
	return sentFromHome
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
	// Good for the navigation the send started, and only that one.
	if (direction !== 'enter') sentFromHome = false
	if (!direction || navigation.willUnload || !document.startViewTransition) return
	if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
	const root = document.documentElement
	// To or from a session under way there is no composer to carry over: Home's would only
	// fade out or in on its own, so the move drops every name and the page cross-fades as
	// one. Leaving, the page says so; arriving, the session's record does (it has sent once
	// it has a workspace), short of the send from Home that just started it.
	const composerOnScreen = Array.from(document.querySelectorAll('[data-session-composer]')).some(
		(el) => getComputedStyle(el).viewTransitionName === 'composer'
	)
	const arrivingId = navigation.to?.url.searchParams.get('session') ?? sessionState.currentSessionId
	const arrivingUnderWay =
		!sentFromHome && !!sessionState.sessions.find((s) => s.id === arrivingId)?.workspace_id
	const plain = direction === 'leave' ? !composerOnScreen : arrivingUnderWay
	root.dataset.homeSessionsTransition = plain ? 'fade' : direction
	return new Promise((resolve) => {
		const transition = document.startViewTransition(async () => {
			resolve()
			await navigation.complete
		})
		// Rejects when the navigation is cancelled or fails; the attribute goes either way.
		const done = () => {
			delete root.dataset.homeSessionsTransition
			sentFromHome = false
		}
		void transition.finished.then(done, done)
	})
}
