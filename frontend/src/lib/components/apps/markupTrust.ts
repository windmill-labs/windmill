import { getContext } from 'svelte'
import { IS_APP_ISOLATED_CONTEXT_KEY, IS_APP_PUBLIC_CONTEXT_KEY } from './types'

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
 * Trust level for markup rendered by a low-code app component. Call during init.
 *
 * An app renders its own author's markup, so it is `trusted` — except on the public
 * surfaces, where the app is distributed to untrusted viewers. There it needs the
 * viewer's approval, unless the app is sandbox-isolated (opaque origin), in which
 * case the markup can't reach the viewer's session and needs no gate.
 */
export function getAppMarkupTrust(): MarkupTrust {
	const isPublic = getContext<boolean | undefined>(IS_APP_PUBLIC_CONTEXT_KEY)
	const isIsolated = getContext<boolean | undefined>(IS_APP_ISOLATED_CONTEXT_KEY)
	return isPublic && !isIsolated ? 'approval' : 'trusted'
}
