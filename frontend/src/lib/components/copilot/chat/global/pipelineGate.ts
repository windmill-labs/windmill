/**
 * Dev-only gate for data pipelines inside AI sessions.
 *
 * Data pipelines are in alpha: a session chat must not build them for regular
 * users yet — it tells the user pipelines will be handled by the chat soon
 * instead. Developers/QA enable the session pipeline surface in their browser
 * with:
 *
 *     localStorage.setItem('wm_dev_session_pipelines', '1')
 *
 * and reload the page. To disable, remove the key or set it to anything else.
 *
 * Only the session surface is gated: the standalone global side-panel chat and
 * the full-page /pipeline editor keep their pipeline support (the eval harness
 * exercises the former in Node, where this gate must never engage). When
 * pipelines ship in sessions, replace every call to
 * `isSessionPipelinesEnabled()` with `true` and delete this file. The
 * references are intentionally narrow (the session system-prompt bullets, the
 * get_instructions/open_preview execution guards, and the session preview
 * router) so the rip-out is a small grep.
 */
const STORAGE_KEY = 'wm_dev_session_pipelines'

export function isSessionPipelinesEnabled(): boolean {
	if (typeof localStorage === 'undefined') return false
	try {
		return localStorage.getItem(STORAGE_KEY) === '1'
	} catch {
		return false
	}
}

/** Tool result returned when a gated session chat still tries a pipeline
 * action — instructs the model to deliver the alpha notice instead. */
export const SESSION_PIPELINES_GATED_MESSAGE =
	'Not available: data pipelines are in alpha and this chat cannot build them yet. Tell the user that data pipelines are in alpha and will be handled by the chat soon. Do not write annotated pipeline scripts manually, and do not build a flow as a substitute.'
