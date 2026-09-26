import type { Component } from 'svelte'

// The ~300 icon modules load on first use rather than up front: statically reached from
// the app shell, they gate every page load in dev. Everything here returns `undefined`
// until they have loaded and then re-runs reactive readers — call from markup or a
// `$derived`, not once into a plain variable.
let icons = $state.raw<typeof import('./index')>()
let loading: Promise<unknown> | undefined

function load() {
	if (!icons)
		loading ??= import('./index').then(
			(m) => (icons = m),
			// Cleared so the next read retries rather than keeping the tab icon-less.
			() => (loading = undefined)
		)
	return icons
}

export function appIconComponent(name: string | undefined): Component | undefined {
	if (!name) return undefined
	return load()?.appIconComponent(name)
}

export function appIconMap(): Record<string, Component> | undefined {
	return load()?.APP_TO_ICON_COMPONENT
}
