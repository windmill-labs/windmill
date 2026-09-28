import { getLocalSetting } from '$lib/utils'

// Module-level because every warm session keeps its own chat, and so its own
// banner, mounted: dismissing one must hide them all.
const DISMISSED_KEY = 'wm_sessions_beta_banner_dismissed'

/** Mutable so the kitchen_sink playground can tune it. */
export const dismissTiming = $state({ pulseMs: 1000 })

let dismissed = $state(getLocalSetting(DISMISSED_KEY) === '1')
let hintSettings = $state(false)
let hintTimer: ReturnType<typeof setTimeout> | undefined

export const sessionsBetaBanner = {
	get dismissed() {
		return dismissed
	},
	/** True for one pulse of the assistant settings button, where "Switch back to
	 * legacy chat" lives once the banner is gone. */
	get hintSettings() {
		return hintSettings
	}
}

export function dismissSessionsBetaBanner() {
	dismissed = true
	try {
		localStorage.setItem(DISMISSED_KEY, '1')
	} catch {}
	hintSettings = true
	clearTimeout(hintTimer)
	hintTimer = setTimeout(() => (hintSettings = false), dismissTiming.pulseMs)
}

/** For the kitchen_sink playground that replays the animation. */
export function resetSessionsBetaBanner() {
	dismissed = false
	hintSettings = false
	clearTimeout(hintTimer)
	try {
		localStorage.removeItem(DISMISSED_KEY)
	} catch {}
}
