import { writable } from 'svelte/store'

// Optional external domain (e.g. "https://app.windmill.dev") used to build
// absolute URLs for user-facing links when the Windmill UI is embedded on a
// different host than the API. Undefined = same-origin (default behavior).
// Intended to be set once at boot by SDK consumers (e.g. windmill-react-sdk).
export const externalDomain = writable<string | undefined>(undefined)

export function withExternalDomain(path: string, domain: string | undefined): string {
	if (!domain) return path
	return `${domain.replace(/\/$/, '')}${path.startsWith('/') ? path : `/${path}`}`
}
