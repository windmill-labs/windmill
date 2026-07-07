import { base } from '$app/paths'
import type { SessionTarget } from './sessionState.svelte'

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

// Force the global sidebar off in the previewed page (the sessions page already
// has its own navigation rail) by setting Windmill's `nomenubar` query flag.
// A session deliberately never switches the global workspaceStore, so the iframe
// must carry the session's effective workspace as `?workspace=` — the logged
// layout reads that param to scope the page, otherwise it renders against the
// navigation workspace (wrong data for a fork-scoped session). Returns a
// relative URL so the iframe stays same-origin.
export function withMenuHidden(url: string, workspaceId?: string): string {
	try {
		const u = new URL(url, 'http://_')
		u.searchParams.set('nomenubar', 'true')
		if (workspaceId) u.searchParams.set('workspace', workspaceId)
		return u.pathname + u.search + u.hash
	} catch {
		return url
	}
}
