import { getContext, setContext } from 'svelte'

const KEY = Symbol('untrusted-images')

/**
 * Marks the subtree as showing text another user wrote to whoever opens it (a shared session):
 * an image in it that points elsewhere would report every reader to that host, so the
 * markdown renderers draw it as a link instead. Every image the subtree can draw from that
 * text must go through {@link MarkdownImage}, which is the only thing standing between it and
 * the reader's browser.
 */
export function setUntrustedImages() {
	setContext(KEY, true)
}

export function hasUntrustedImages(): boolean {
	return getContext<boolean | undefined>(KEY) === true
}

/** Sources that reach no third party: embedded bytes, or the Windmill server itself. */
export function isLocalImageSrc(src: string | undefined): boolean {
	if (!src) return false
	try {
		const url = new URL(src, window.location.href)
		return (
			url.protocol === 'data:' || url.protocol === 'blob:' || url.origin === window.location.origin
		)
	} catch {
		return false
	}
}
