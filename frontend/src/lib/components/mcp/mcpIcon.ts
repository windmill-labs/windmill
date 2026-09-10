/** One entry of an MCP `icons` array, as the spec defines it. */
export type McpIcon = {
	src?: unknown
	mimeType?: unknown
	sizes?: unknown
	theme?: unknown
}

// The chosen src is persisted on a transcript row, and the whole chat record is
// re-cloned into IndexedDB on every save — so an icon drawn at 14px is held to a size
// that suits one. (`imageUrl` earns its out-of-band store by being a screenshot.)
const MAX_ICON_SRC_CHARS = 8_000

/**
 * The icon source to render, chosen from what an MCP server published.
 *
 * **`data:` only.** The spec also permits an HTTPS src, but rendering one makes the
 * user's browser fetch from a host the server names, disclosing their IP, user agent
 * and the moment they looked — and a per-user URL turns it into a read receipt. COEP
 * `require-corp` does not prevent that: it blocks the *response*, so the request has
 * already left. That is the same disclosure the favicon lookup was removed for, and
 * against an arbitrary host rather than one known party. A `data:` src carries its
 * bytes over the MCP connection the user already made and fetches nothing.
 *
 * The rest is the spec's own list, enforced rather than trusted, since `src` is chosen
 * by a third party: no SVG (it can carry script), and a data URI judged by the type it
 * declares rather than the sibling `mimeType`, which the spec calls advisory.
 */
export function pickMcpIconSrc(icons: unknown): string | undefined {
	if (!Array.isArray(icons)) return undefined
	return icons.find((icon): icon is McpIcon => isRenderable(icon))?.src as string | undefined
}

function isRenderable(icon: unknown): boolean {
	const src = (icon as McpIcon | null)?.src
	if (typeof src !== 'string' || src.length > MAX_ICON_SRC_CHARS) return false
	const lower = src.toLowerCase()
	if (!lower.startsWith('data:image/')) return false
	return ['png', 'jpeg', 'jpg', 'webp'].some((type) => lower.startsWith(`data:image/${type}`))
}
