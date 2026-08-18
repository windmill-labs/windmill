import { fromStore } from 'svelte/store'
import { base } from '$lib/base'
import { oauthStore } from '$lib/stores'

const OAUTH_WINDOW = 'windmill_supabase_oauth'
const CONNECT_URL = `${base}/api/oauth/connect/supabase_wizard`

/**
 * The Supabase authorization leg, driven from a popup.
 *
 * A full-page redirect unmounts whatever opened it, so a user who stops to create a Supabase
 * account lands on their dashboard with nothing left pointing back. Keeping the flow in a
 * popup keeps the host on screen, and keeps the window ours to steer: after they sign up we
 * send the same popup back through the connect endpoint and consent follows.
 */
export function useSupabaseOauth(
	opts: {
		onPopupBlocked?: () => void
		/**
		 * Where popups are blocked, navigate this tab instead of opening a new one. Only for
		 * hosts that can be resumed afterwards -- a caller whose state dies with the page (a
		 * half-filled form) must leave this off and keep the user where they are.
		 */
		redirectIfBlocked?: boolean
		/** Even the new tab was refused, so the caller has to say so rather than sit loading. */
		onFallbackBlocked?: () => void
		/** The window went away without authorizing; the caller can drop its own waiting state. */
		onAbandoned?: () => void
		/**
		 * Authorization came back and the token is in the store. Reported like the failures
		 * above so a caller does not have to watch `authed` to find out. Fires on any successful
		 * authorization, this caller's or another's -- every instance listens on the same window
		 * -- so a caller that acts on it has to know it was the one waiting.
		 */
		onAuthed?: () => void
	} = {}
) {
	const oauth = fromStore(oauthStore)
	let pending = $state(false)
	let win: Window | null = null
	let abandonWatch: ReturnType<typeof setInterval> | undefined = undefined

	$effect(() => {
		function onMessage(e: MessageEvent) {
			if (e.origin !== window.location.origin || e.data?.type !== 'supabase_oauth') return
			oauthStore.set(e.data.res)
			pending = false
			clearInterval(abandonWatch)
			win?.close()
			opts.onAuthed?.()
		}
		window.addEventListener('message', onMessage)
		return () => {
			window.removeEventListener('message', onMessage)
			clearInterval(abandonWatch)
		}
	})

	/**
	 * Nothing arrives if the user closes the window, denies consent, or wanders off to create
	 * an account first -- which is a link this flow deliberately offers. Watch for the window
	 * going away, so the button comes back instead of staying disabled until a page reload.
	 */
	function watchForAbandon() {
		clearInterval(abandonWatch)
		abandonWatch = setInterval(() => {
			if (!win || win.closed) {
				clearInterval(abandonWatch)
				pending = false
				opts.onAbandoned?.()
			}
		}, 500)
	}

	return {
		get token(): string | undefined {
			return oauth.current?.access_token
		},
		get authed(): boolean {
			return !!oauth.current?.access_token
		},
		get pending(): boolean {
			return pending
		},
		/** Opens (or re-points) the popup, falling back to a new tab where popups are blocked. */
		connect() {
			win = window.open(CONNECT_URL, OAUTH_WINDOW, 'width=600,height=820')
			if (!win) {
				opts.onPopupBlocked?.()
				if (opts.redirectIfBlocked) {
					window.location.href = CONNECT_URL
					return
				}
				// No `noopener`: the callback hands the token back through `window.opener`, and
				// severing that is what would leave the host waiting forever. The URL is our own
				// origin, so there is nothing to protect against here.
				win = window.open(CONNECT_URL, '_blank')
				if (!win) {
					opts.onFallbackBlocked?.()
					return
				}
			}
			pending = true
			watchForAbandon()
		}
	}
}
