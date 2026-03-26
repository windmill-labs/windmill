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
