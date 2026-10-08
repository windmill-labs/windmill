/** The browser extension framing this page, if one does. `ancestorOrigins` is set by the
 * browser, so a framing web page cannot claim to be an extension. */
export function extensionParentOrigin(): string | undefined {
	if (typeof window === 'undefined' || window.parent === window) return undefined
	const origin = window.location.ancestorOrigins?.[0]
	return origin?.startsWith('chrome-extension://') ? origin : undefined
}

/** The only page the extension's side panel shows once signed in to a workspace. */
export const EXTENSION_PANEL_PATH = '/sessions/browser'
