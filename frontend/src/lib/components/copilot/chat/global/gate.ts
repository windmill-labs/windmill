/**
 * Beta opt-out gate for AI Sessions (and the Global AI chat mode).
 *
 * Sessions ship enabled by default. The opt-out lives in this browser's
 * localStorage; nothing in the UI sets it any more, so only browsers that
 * opted out earlier keep the legacy docked chat, whose banner switches them
 * back. The toggle does a full page reload: every call site reads the gate
 * once at init, so a live flip would leave the UI half-switched.
 *
 * When the beta ends, replace every call to `isGlobalAiEnabled()` with `true`
 * and delete this file. The references are intentionally narrow (chat mode
 * visibility, custom prompt settings, and the `change_mode` tool enum) so the
 * rip-out is a small grep.
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

const OPERATOR_SESSIONS_KEY = 'wm_operator_ai_sessions'

/**
 * Operators get AI sessions only in a browser that opted in, until the operator experience
 * has been tested; everywhere else they keep the docked chat and `/sessions` refuses them.
 * Set with `localStorage.setItem('wm_operator_ai_sessions', '1')` and reload. To ship it,
 * delete this and `sessionsAllowedFor`, and drop the `isOperator` argument from every caller.
 */
function operatorSessionsEnabled(): boolean {
	if (typeof localStorage === 'undefined') return false
	try {
		return localStorage.getItem(OPERATOR_SESSIONS_KEY) === '1'
	} catch {
		return false
	}
}

/** Whether this user's role may use AI sessions at all. */
export function sessionsAllowedFor(isOperator: boolean | undefined): boolean {
	return !isOperator || operatorSessionsEnabled()
}

/**
 * Whether an AI entry point hands off to a session instead of driving the docked
 * chat. Deliberately the same condition as the root layout's `disableAi`, so a
 * caller falling back on `false` always has a mounted pane to fall back to.
 */
export function prefersSessionHandoff(isOperator: boolean | undefined): boolean {
	return isGlobalAiEnabled() && sessionsAllowedFor(isOperator)
}

/** Clear the opt-out, then hard-reload so every gated site re-reads it. */
export function clearSessionsBetaOptOut(target: string) {
	// Navigate even when persistence throws (quota, private browsing) — the
	// button must not be a silent no-op. The reload then shows the unchanged
	// mode, which is the honest feedback that the toggle didn't stick.
	let persisted = true
	try {
		localStorage.removeItem(OPT_OUT_KEY)
	} catch {
		persisted = false
	}
	// Anonymous usage counter on the shared feature_usage channel. The buffer's
	// pagehide flush + keepalive fetch carry it across the hard navigation below.
	if (persisted) {
		logFeatureUsage('ai_session', 'beta_optin')
	}
	window.location.href = target
}
