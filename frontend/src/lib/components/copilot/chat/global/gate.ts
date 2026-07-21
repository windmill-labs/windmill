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
import { get } from 'svelte/store'
import { workspaceStore } from '$lib/stores'
import { WorkspaceService } from '$lib/gen'
import { randomUUID } from '$lib/utils/uuid'

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
export async function setSessionsBetaOptOut(optOut: boolean, target: string) {
	try {
		if (optOut) localStorage.setItem(OPT_OUT_KEY, '1')
		else localStorage.removeItem(OPT_OUT_KEY)
	} catch {
		return
	}
	// Best-effort telemetry on the same channel as chat messages (ai_chat_usage).
	// Awaited with a cap: the hard navigation below would abort a fire-and-forget
	// request, but a hanging API must not trap the user on the old mode either.
	const workspace = get(workspaceStore)
	if (workspace) {
		await Promise.race([
			WorkspaceService.logAiChat({
				workspace,
				requestBody: {
					session_id: randomUUID(),
					provider: 'none',
					model: 'none',
					mode: optOut ? 'sessions_beta_optout' : 'sessions_beta_optin'
				}
			}).catch(() => {}),
			new Promise((resolve) => setTimeout(resolve, 1500))
		])
	}
	window.location.href = target
}
