/**
 * A hostname's favicon, from Google's public favicon service.
 *
 * This discloses each consulted hostname to a third party from the user's
 * browser — an accepted tradeoff, and blocked or air-gapped environments must
 * degrade to a local icon through the caller's `onerror`.
 *
 * Hit gstatic directly rather than `www.google.com/s2/favicons`: the app is
 * served with COEP require-corp, and the s2 redirect hop carries no
 * Cross-Origin-Resource-Policy header, so the browser blocks the image. The
 * gstatic endpoint itself responds with CORP: cross-origin.
 */
export function faviconUrl(hostname: string, size = 64): string {
	return `https://t3.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=https://${encodeURIComponent(hostname)}&size=${size}`
}
