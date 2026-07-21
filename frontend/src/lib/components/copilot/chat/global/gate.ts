/**
 * Beta opt-out gate for AI Sessions (and the Global AI chat mode).
 *
 * Sessions ship enabled by default. Users can switch back to the legacy
 * docked chat from the beta banner under the session chat, which stores an
 * opt-out in this browser's localStorage; the mirror banner in the legacy
 * chat switches back. Both toggles do a full page reload: every call site
 * reads the gate once at init, so a live flip would leave the UI half-switched.
 *
 * When the beta ends, replace every call to `isGlobalAiEnabled()` with `true`
 * and delete this file. The references are intentionally narrow (chat mode
 * visibility, custom prompt settings, the `change_mode` tool enum, and the
 * AI skills workspace settings tab) so the rip-out is a small grep.
 */
import { logFeatureUsage } from '$lib/utils/featureUsage'

const OPT_OUT_KEY = 'wm_sessions_beta_optout'

export function isGlobalAiEnabled(): boolean {
	if (typeof localStorage === 'undefined') return false
	try {
		return localStorage.getItem(OPT_OUT_KEY) !== '1'
	} catch {
		return false
	}
}

/** Persist the opt-out choice, then hard-reload so every gated site re-reads it. */
export function setSessionsBetaOptOut(optOut: boolean, target: string) {
	// Navigate even when persistence throws (quota, private browsing) — the
	// button must not be a silent no-op. The reload then shows the unchanged
	// mode, which is the honest feedback that the toggle didn't stick.
	let persisted = true
	try {
		if (optOut) localStorage.setItem(OPT_OUT_KEY, '1')
		else localStorage.removeItem(OPT_OUT_KEY)
	} catch {
		persisted = false
	}
	// Anonymous usage counter on the shared feature_usage channel. The buffer's
	// pagehide flush + keepalive fetch carry it across the hard navigation below.
	if (persisted) {
		logFeatureUsage('ai_session', optOut ? 'beta_optout' : 'beta_optin')
	}
	window.location.href = target
}
