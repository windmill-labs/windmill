import { UserService } from '$lib/gen'
import { noteSessionEmail } from '$lib/onboardingProfile'

// An account that entered through an invite's sign-in link has no credentials of its own
// (`login_type` pending_oauth) until it sets a password or an OAuth login adopts it. Every
// surface that nags about it — the sidebar banner, the Settings entry, the mobile menu —
// reads this one flag and opens the one modal, so finishing on any of them clears all.
let pending = $state(false)
let open = $state(false)
let inflight: Promise<void> | undefined
// Bumped on sign-out so a lookup still in flight for the previous account cannot land
// its answer on the next one.
let generation = 0

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
	/** Forget the signed-out account; the module outlives a same-tab sign-out. */
	reset() {
		generation++
		inflight = undefined
		pending = false
		open = false
	},
	/**
	 * Re-read the login type. Shared across callers mounting at the same time so the
	 * sidebar's several readers cost one request, not one each.
	 */
	refresh(): Promise<void> {
		if (!inflight) {
			const started = generation
			inflight = UserService.globalWhoami()
				.then((me) => {
					if (started !== generation) return
					noteSessionEmail(me.email)
					pending = me.login_type === 'pending_oauth'
					// The finish-setup marker is set for one provider round trip; a SAML one
					// never comes back through the page that clears it, so it is dropped as
					// soon as the account is known to be adopted, not left to refuse a later
					// sign-in as a mismatch.
					if (!pending) {
						document.cookie = 'finish_setup=; path=/; max-age=0; SameSite=Lax'
					}
				})
				.catch(() => {
					if (started === generation) pending = false
				})
				.finally(() => {
					if (started === generation) inflight = undefined
				})
		}
		return inflight
	}
}
