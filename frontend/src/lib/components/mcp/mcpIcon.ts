/** One entry of an MCP `icons` array, as the spec defines it. */
export type McpIcon = {
	src?: unknown
	mimeType?: unknown
	sizes?: unknown
	theme?: unknown
}

// A data URI rides in the request that renders it and is persisted on a transcript
// row, so an oversized one is refused rather than carried.
const MAX_ICON_SRC_CHARS = 64_000

/**
 * The icon source to render, chosen from what an MCP server published.
 *
 * The `src` is chosen by the server, so the spec's rules are enforced here rather
 * than trusted: only `https:` and `data:` (it names `javascript:`, `file:`, `ftp:`
 * and `ws:` as the schemes to reject), and no SVG — it can carry script, and
 * sanitising it is not worth the icon. `mimeType` is advisory, so a data URI is
 * judged on the type it actually declares.
 *
 * An `https:` src is accepted but will usually fail to load: the app is served with
 * COEP require-corp and a third-party image without a Cross-Origin-Resource-Policy
 * header is blocked. `McpServerIcon` falls through to the next source when it does.
 */
export function pickMcpIconSrc(icons: unknown, dark = false): string | undefined {
	if (!Array.isArray(icons)) return undefined
	const usable = icons.filter((icon): icon is McpIcon => isRenderable(icon))
	// `theme` names the background the icon was drawn for; an untagged icon suits either.
	const themed = usable.find((icon) => icon.theme === (dark ? 'dark' : 'light'))
	const untagged = usable.find((icon) => icon.theme === undefined)
	return (themed ?? untagged ?? usable[0])?.src as string | undefined
}

function isRenderable(icon: unknown): boolean {
	const src = (icon as McpIcon | null)?.src
	if (typeof src !== 'string' || src.length > MAX_ICON_SRC_CHARS) return false
	const lower = src.toLowerCase()
	if (lower.startsWith('https://')) return !lower.endsWith('.svg')
	if (!lower.startsWith('data:image/')) return false
	// Judge the data URI by the type it declares, not by the sibling `mimeType`.
	return ['png', 'jpeg', 'jpg', 'webp'].some((type) => lower.startsWith(`data:image/${type}`))
}
