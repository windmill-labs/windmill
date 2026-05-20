import { get } from 'svelte/store'
import { hubBaseUrlStore } from './stores'

export function isValidLogoutRedirect(url: string): boolean {
	if (url.startsWith('/') && !url.startsWith('//')) {
		return true
	}
	try {
		const parsed = new URL(url)
		const host = parsed.hostname
		if (host === 'windmill.dev' || host.endsWith('.windmill.dev')) {
			return true
		}
		const hubBaseUrl = get(hubBaseUrlStore)
		try {
			const hubHost = new URL(hubBaseUrl).hostname
			if (host === hubHost) {
				return true
			}
		} catch {}
	} catch {}
	return false
}

/**
 * Reduces a redirect target to a safe same-origin relative path.
 *
 * Returns the path (`/foo?bar#baz`) when the input is either:
 *   - already an absolute-but-relative path (`/foo`) that isn't protocol-relative
 *     (`//evil.com`) or a back-slash trick (`/\\evil.com`), or
 *   - a full URL whose origin matches the current page's origin.
 *
 * Returns `null` for anything else (cross-origin URLs, protocol-relative paths,
 * malformed input). Used to sanitize values flowing into `RelayState`,
 * `localStorage.rd`, and post-login redirects so we don't either lose a valid
 * same-origin deep link or open-redirect into a foreign origin.
 */
export function toSameOriginRelativePath(rd: string | null | undefined): string | null {
	if (!rd) return null
	if (rd.length > 2048) return null
	if (hasControlChar(rd)) return null
	if (rd.startsWith('/')) {
		if (rd.startsWith('//') || rd.startsWith('/\\')) return null
		return rd
	}
	if (typeof window === 'undefined') return null
	try {
		const url = new URL(rd, window.location.origin)
		if (url.origin !== window.location.origin) return null
		const path = url.pathname + url.search + url.hash
		return path || '/'
	} catch {
		return null
	}
}

function hasControlChar(s: string): boolean {
	for (let i = 0; i < s.length; i++) {
		const c = s.charCodeAt(i)
		if (c < 0x20 || c === 0x7f) return true
	}
	return false
}
