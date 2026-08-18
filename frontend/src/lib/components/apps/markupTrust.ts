import { BROWSER } from 'esm-env'
import { getContext } from 'svelte'
import { IS_APP_PUBLIC_CONTEXT_KEY } from './types'

/**
 * How a component should treat runnable-authored `html`/`svg` markup.
 *
 * - `sanitize`: strip scripts/handlers first. The default everywhere outside an app
 *   (job results, previews, flows), where the markup's author and its viewer are
 *   different users.
 * - `trusted`: render verbatim, so dynamic svg/html keeps working.
 * - `approval`: render verbatim, but only after the viewer opts in.
 */
export type MarkupTrust = 'sanitize' | 'trusted' | 'approval'

/**
 * True when this document has an opaque origin, i.e. it is the app sandbox's iframe
 * and markup rendered here cannot reach the viewer's Windmill session.
 *
 * Read the real origin rather than a `wm_embed`/framing signal: those are set by
 * whoever framed us, so a hostile page could claim to be the sandbox while running
 * on the Windmill origin with the viewer's cookies. The origin can't be faked the
 * same way — anyone who makes it opaque has, by doing so, given up the session
 * access this gate exists to protect.
 */
function isOpaqueOrigin(): boolean {
	return BROWSER && window.origin === 'null'
}

/**
 * Trust level for markup rendered by a low-code app component. Call during init.
 *
 * An app renders its own author's markup, so it is `trusted` — except on the public
 * surfaces, where the app is distributed to untrusted viewers. There it needs the
 * viewer's approval, unless the app is sandbox-isolated, in which case the markup
 * can't reach the viewer's session and needs no gate.
 */
export function getAppMarkupTrust(): MarkupTrust {
	const isPublic = getContext<boolean | undefined>(IS_APP_PUBLIC_CONTEXT_KEY)
	if (!isPublic) return 'trusted'
	return isOpaqueOrigin() ? 'trusted' : 'approval'
}
