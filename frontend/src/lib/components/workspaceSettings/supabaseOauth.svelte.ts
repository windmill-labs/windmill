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
export function useSupabaseOauth(opts: { onPopupBlocked?: () => void } = {}) {
	const oauth = fromStore(oauthStore)
	let pending = $state(false)
	let win: Window | null = null

	$effect(() => {
		function onMessage(e: MessageEvent) {
			if (e.origin !== window.location.origin || e.data?.type !== 'supabase_oauth') return
			oauthStore.set(e.data.res)
			pending = false
			win?.close()
		}
		window.addEventListener('message', onMessage)
		return () => window.removeEventListener('message', onMessage)
	})

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
		/** Opens (or re-points) the popup. Falls back to a redirect where popups are blocked. */
		connect() {
			win = window.open(CONNECT_URL, OAUTH_WINDOW, 'width=600,height=820')
			if (!win) {
				opts.onPopupBlocked?.()
				window.location.href = CONNECT_URL
				return
			}
			pending = true
		}
	}
}
