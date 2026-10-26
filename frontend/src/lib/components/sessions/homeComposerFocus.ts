import type { OnNavigate } from '@sveltejs/kit'

// Leaving AI Sessions for Home with the composer focused hands that focus on to Home's
// composer, so typing carries on across the move.

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

/** For SvelteKit's `onNavigate`: records whether the composer had focus as the page left. */
export function recordComposerFocus(navigation: OnNavigate): void {
	composerFocused = composerFocusAfter(
		navigation.from?.url.pathname,
		navigation.to?.url.pathname,
		document.activeElement?.closest('[data-session-composer]') != null
	)
}
