const SAFE_PROTOCOLS = ['http:', 'https:', 'mailto:']

/**
 * The href a link built from untrusted content may carry, or undefined for one that must be
 * dropped: `javascript:`, `data:` and any other non-navigating scheme.
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
