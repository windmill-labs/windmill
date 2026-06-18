import { get } from 'svelte/store'
import { BROWSER } from 'esm-env'
import { wsBaseUrlStore } from './stores'

export function buildWsUrl(path: string): string {
	const override = get(wsBaseUrlStore)
	if (override) {
		const base = override.endsWith('/') ? override.slice(0, -1) : override
		return `${base}${path}`
	}
	if (!BROWSER) {
		return `ws://localhost:3003${path}`
	}
	const wsProtocol = window.location.protocol === 'https:' ? 'wss' : 'ws'
	return `${wsProtocol}://${window.location.host}${path}`
}
