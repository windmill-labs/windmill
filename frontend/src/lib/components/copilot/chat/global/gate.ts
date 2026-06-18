/**
 * Dev-only gate for the Global AI chat mode.
 *
 * While the mode is being iterated on, it should not be visible to regular
 * users. Developers/QA enable it in their browser with:
 *
 *     localStorage.setItem('wm_dev_global_ai', '1')
 *
 * and reload the page. To disable, remove the key or set it to anything else.
 *
 * When the mode is ready to ship to everyone, replace every call to
 * `isGlobalAiEnabled()` with `true` and delete this file. The references are
 * intentionally narrow (chat mode visibility, custom prompt settings, the
 * `change_mode` tool enum, and the `/global_drafts` dev route) so the rip-out
 * is a small grep.
 */
const STORAGE_KEY = 'wm_dev_global_ai'

export function isGlobalAiEnabled(): boolean {
	if (typeof localStorage === 'undefined') return false
	try {
		return localStorage.getItem(STORAGE_KEY) === '1'
	} catch {
		return false
	}
}
