// Optional external domain (e.g. "https://app.windmill.dev") used to build
// absolute URLs for user-facing links when the Windmill UI is embedded on a
// different host than the API. Undefined = same-origin (default behavior).
// Intended to be set once at boot by SDK consumers (e.g. windmill-react-sdk),
// before any component that builds links has rendered — there is no
// reactivity, so changing it after mount won't update already-rendered hrefs.
let externalDomain: string | undefined = undefined

export function setExternalDomain(domain: string | undefined): void {
	externalDomain = domain
}

export function withExternalDomain(path: string): string {
	if (!externalDomain) return path
	const trimmed = externalDomain.replace(/\/$/, '')
	return `${trimmed}${path.startsWith('/') ? path : `/${path}`}`
}
