import type { Component } from 'svelte'
import { isChunkLoadError, reloadForStaleChunk } from '$lib/utils/staleChunkReload'

// The ~300 icon modules are a dynamic import: statically reached from the app shell, they
// gate every page load in dev. Everything here returns `undefined` until they have loaded
// and then re-runs reactive readers — call from markup or a `$derived`, not once into a
// plain variable.
let icons = $state.raw<typeof import('./index')>()
let loading: Promise<unknown> | undefined

function load() {
	if (!icons)
		loading ??= import('./index').then(
			(m) => (icons = m),
			// Cleared so the next read retries rather than keeping the tab icon-less.
			(e) => {
				loading = undefined
				if (isChunkLoadError(String(e?.message))) reloadForStaleChunk()
			}
		)
	return icons
}

// Started when a module that shows icons is evaluated, in parallel with the page's own
// chunks, rather than on the first read during render, which would add a round trip.
if (typeof window !== 'undefined') load()

export function appIconComponent(name: string | undefined): Component | undefined {
	if (!name) return undefined
	return load()?.appIconComponent(name)
}

export function appIconMap(): Record<string, Component> | undefined {
	return load()?.APP_TO_ICON_COMPONENT
}
