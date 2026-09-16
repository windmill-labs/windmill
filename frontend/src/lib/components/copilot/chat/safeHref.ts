const SAFE_PROTOCOLS = ['http:', 'https:', 'mailto:']

/**
 * The href a rendered markdown link may carry, or undefined for one that must be dropped.
 *
 * Markdown reaches the chat renderers from a model, and from another member for a shared
 * artifact, and `svelte-exmarkdown` passes `javascript:` and `data:` hrefs through untouched.
 * Relative links resolve against `base` (the page), so they stay.
 */
export function safeHref(href: string | undefined, base: string): string | undefined {
	if (!href) return undefined
	try {
		return SAFE_PROTOCOLS.includes(new URL(href, base).protocol) ? href : undefined
	} catch {
		return undefined
	}
}
