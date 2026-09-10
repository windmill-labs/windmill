import { UserService } from '$lib/gen'

// An account that entered through an invite's sign-in link has no credentials of its own
// (`login_type` pending_oauth) until it sets a password or an OAuth login adopts it. Every
// surface that nags about it — the sidebar banner, the Settings entry, the mobile menu —
// reads this one flag and opens the one modal, so finishing on any of them clears all.
let pending = $state(false)
let open = $state(false)
let inflight: Promise<void> | undefined

export const accountSetup = {
	/** Whether the signed-in account still has to set a password or connect a sign-in. */
	get pending() {
		return pending
	},
	/** The finish-setup modal, hosted once by the logged-in layout. */
	get open() {
		return open
	},
	set open(v: boolean) {
		open = v
	},
	/**
	 * Re-read the login type. Shared across callers mounting at the same time so the
	 * sidebar's several readers cost one request, not one each.
	 */
	refresh(): Promise<void> {
		if (!inflight) {
			inflight = UserService.globalWhoami()
				.then((me) => {
					pending = me.login_type === 'pending_oauth'
				})
				.catch(() => {
					pending = false
				})
				.finally(() => {
					inflight = undefined
				})
		}
		return inflight
	}
}
