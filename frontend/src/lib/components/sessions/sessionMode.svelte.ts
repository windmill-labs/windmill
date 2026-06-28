import { BROWSER } from 'esm-env'
import { base } from '$app/paths'
import type { SessionTarget } from './sessionState.svelte'

// Session mode is an optional layout wrapper, not a route: it can be toggled
// on over whatever Windmill page is currently shown. The flag is sticky across
// reloads/navigations so the wrapped layout survives a refresh.
const STORAGE_KEY = 'wm_session_mode'

function readInitial(): boolean {
	if (!BROWSER) return false
	try {
		return localStorage.getItem(STORAGE_KEY) === '1'
	} catch {
		return false
	}
}

export const sessionLayout = $state<{ on: boolean }>({ on: readInitial() })

export function setSessionMode(on: boolean): void {
	sessionLayout.on = on
	if (!BROWSER) return
	try {
		if (on) localStorage.setItem(STORAGE_KEY, '1')
		else localStorage.removeItem(STORAGE_KEY)
	} catch {
		// localStorage can throw in private mode — the in-memory flag still drives the layout.
	}
}

export function toggleSessionMode(): void {
	setSessionMode(!sessionLayout.on)
}

// Maps a session's editor target to the canonical full-page Windmill route.
// Clicking a session navigates the embedded panel here; non-editor targets
// (or none) return undefined so the panel just keeps its current page.
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
