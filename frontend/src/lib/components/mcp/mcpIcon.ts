/** One entry of an MCP `icons` array, as the spec defines it. Not `McpIcon`: that
 * names the Windmill-shipped MCP logo component in `$lib/components/icons`. */
export type PublishedIcon = {
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
 * The icon source to render, chosen from what an MCP server published. `src` comes
 * from a third party, so the spec's rules are enforced here rather than trusted.
 *
 * `data:` only: an HTTPS src would make the browser fetch from a host the server
 * names, and COEP blocks the response, not the request. No SVG — it can carry script.
 * A data URI is judged by the type it declares; `mimeType` is advisory.
 */
export function pickMcpIconSrc(icons: unknown): string | undefined {
	if (!Array.isArray(icons)) return undefined
	return icons.find((icon): icon is PublishedIcon => isRenderable(icon))?.src as string | undefined
}

function isRenderable(icon: unknown): boolean {
	const src = (icon as PublishedIcon | null)?.src
	if (typeof src !== 'string' || src.length > MAX_ICON_SRC_CHARS) return false
	const lower = src.toLowerCase()
	if (!lower.startsWith('data:image/')) return false
	return ['png', 'jpeg', 'jpg', 'webp'].some((type) => lower.startsWith(`data:image/${type}`))
}
