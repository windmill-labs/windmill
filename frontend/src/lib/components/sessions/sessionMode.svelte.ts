import { BROWSER } from 'esm-env'
import { base } from '$app/paths'
import type { SessionTarget } from './sessionState.svelte'

// The sessions page previews a live Windmill page in an iframe. Each session
// remembers which URL to show: a session opened from a page captures the page
// the user was on ("the current view becomes the session's target"); otherwise
// the preview falls back to the session's editor target, then the home page.
const STORAGE_KEY = 'wm_session_preview_urls'

function readInitial(): Record<string, string> {
	if (!BROWSER) return {}
	try {
		const raw = localStorage.getItem(STORAGE_KEY)
		return raw ? (JSON.parse(raw) as Record<string, string>) : {}
	} catch {
		return {}
	}
}

const previewUrls = $state<Record<string, string>>(readInitial())

function persist(): void {
	if (!BROWSER) return
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(previewUrls))
	} catch {
		// localStorage can throw in private mode — the in-memory map still drives the preview.
	}
}

// Remember the page a session should preview. Called when a session is opened
// from a Windmill page so the preview keeps showing that page.
export function captureSessionView(sessionId: string, url: string): void {
	previewUrls[sessionId] = url
	persist()
}

// Maps a session's editor target to the canonical full-page Windmill route.
export function sessionTargetHref(target: SessionTarget | undefined): string | undefined {
	if (!target) return undefined
	const seg =
		target.kind === 'script'
			? 'scripts/edit'
			: target.kind === 'flow'
				? 'flows/edit'
				: target.kind === 'raw_app'
					? 'apps_raw/edit'
					: undefined
	if (!seg) return undefined
	return `${base}/${seg}/${target.path}`
}

// The URL the sessions page should iframe for a session: the explicitly
// captured view, else its editor target's route, else the home page.
export function sessionPreviewUrl(
	session: { id: string; target?: SessionTarget } | undefined
): string {
	if (!session) return `${base}/`
	return previewUrls[session.id] ?? sessionTargetHref(session.target) ?? `${base}/`
}

// Force the global sidebar off in the previewed page (the sessions page already
// has its own navigation rail) by setting Windmill's `nomenubar` query flag.
// Returns a relative URL so the iframe stays same-origin.
export function withMenuHidden(url: string): string {
	try {
		const u = new URL(url, 'http://_')
		u.searchParams.set('nomenubar', 'true')
		return u.pathname + u.search + u.hash
	} catch {
		return url
	}
}
