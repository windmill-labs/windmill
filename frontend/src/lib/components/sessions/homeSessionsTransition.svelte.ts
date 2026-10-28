import type { OnNavigate } from '@sveltejs/kit'
import { tick } from 'svelte'

// A send from Home keeps the composer in sight: the browser snapshots the page before and
// after the navigation to the session the send started, and glides the element that carries
// the `composer` view-transition-name (AIChatDisplay names it) from Home to its place under the
// session's first message; the rest of the page cross-fades. Any other move between Home and
// a session is one cross-fade: a session under way has a composer of its own, not Home's. The
// timing lives in app.css, keyed on the `data-home-sessions-transition` attribute set here for
// the length of the move.

const HOME = '/'
const SESSIONS = '/sessions'

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
	const glides = sentFromHome && from === HOME && to === SESSIONS
	// Good for the navigation the send started, and only that one.
	if (!glides) sentFromHome = false
	const between = (from === HOME && to === SESSIONS) || (from === SESSIONS && to === HOME)
	if (!between || navigation.willUnload || !document.startViewTransition) return
	if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
	const root = document.documentElement
	root.dataset.homeSessionsTransition = glides ? 'glide' : 'fade'
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

/** For the Workspace ⇄ AI Sessions switch, which stays on the page: runs `swap` as a
 * view transition in which the sidebar's list comes in from the side of the button pressed.
 * The name and timing live in app.css, keyed on `data-home-side-transition`: set only for the
 * switch, so a move to a session page does not pick the name up.
 *
 * Resolves once the swap is on the page and the click that asked for it has played out. */
export function swapHomeSide(to: 'workspace' | 'sessions', swap: () => void): Promise<void> {
	if (
		typeof document === 'undefined' ||
		!document.startViewTransition ||
		window.matchMedia('(prefers-reduced-motion: reduce)').matches
	) {
		swap()
		return new Promise((resolve) => setTimeout(resolve, 0))
	}
	const root = document.documentElement
	root.dataset.homeSideTransition = to
	const transition = document.startViewTransition(async () => {
		swap()
		await tick()
	})
	const done = () => delete root.dataset.homeSideTransition
	void transition.finished.then(done, done)
	return transition.updateCallbackDone.then(
		() => undefined,
		() => undefined
	)
}
