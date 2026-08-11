//  https://github.com/jsdelivr/data.jsdelivr.com

import pLimit from 'p-limit'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'

// Backend proxy fallback functions
const getBackendProxyUrl = () => {
	const workspace = get(workspaceStore)
	if (!workspace) {
		throw new Error('No workspace available')
	}
	return `/api/w/${workspace}/npm_proxy`
}

const backendProxyApi = async <T>(endpoint: string, resLimit: ResLimit): Promise<T | Error> => {
	if (isOverlimit(resLimit)) {
		console.warn(
			`Exceeded limit of types downloaded for the needs of the assistant fetching: ${endpoint}, ${resLimit.usage}`
		)
		return new Error('Exceeded limit of 100MB of data downloaded.')
	}

	try {
		const baseUrl = getBackendProxyUrl()
		const url = `${baseUrl}${endpoint}`

		// `await`, not a bare `return`: an async function adopts a returned promise after
		// leaving the try block, so a rejection would escape the catch below.
		return await limit(() =>
			fetch(url, { credentials: 'include' }).then((res) => {
				if (res.ok) {
					return res.text().then((text) => {
						resLimit.usage += text.length
						console.log('resLimit (backend proxy)', url, resLimit.usage)
						return JSON.parse(text) as T
					}) as Promise<T | Error>
				} else {
					return new Error('Backend proxy request failed')
				}
			})
		)
	} catch (e) {
		// Keep the cause: where the proxy is the only reachable source, this is the sole
		// report of a failed acquisition, since callers only test the result for `Error`.
		console.warn(`Backend proxy request to ${endpoint} failed`, e)
		return new Error(`Backend proxy not available: ${e}`)
	}
}

let registryConfigured: Promise<boolean> | undefined

/**
 * Whether the instance points npm at a registry of its own. Asked once per session: it is
 * an instance setting, and every module acquired would otherwise re-ask. Only a definitive
 * answer is kept — a transient failure, or no workspace to ask under yet, would otherwise
 * pin the whole session to the public CDN, which is the wrong source on such an instance.
 */
async function isRegistryConfigured(): Promise<boolean> {
	if (!get(workspaceStore)) return false
	registryConfigured ??= (async () => {
		try {
			const res = await fetch(`${getBackendProxyUrl()}/config`, { credentials: 'include' })
			if (!res.ok) throw new Error(`${res.status} from the npm proxy`)
			return !!(await res.json())?.registry_configured
		} catch (e) {
			console.warn('Could not read the npm proxy configuration, using the public CDN', e)
			registryConfigured = undefined
			return false
		}
	})()
	return registryConfigured
}

/**
 * Run the two sources in the order the instance's configuration calls for. With a private
 * registry set it is authoritative: jsdelivr does not carry internal packages, asking it
 * discloses their names, and a public package sharing a name answers with the wrong types.
 * Either way the other source stays as fallback, so neither a CDN outage nor a misconfigured
 * proxy leaves the editor with no types at all.
 */
async function fromPreferredSource<T>(
	viaCdn: () => Promise<T | Error>,
	viaProxy: () => Promise<T | Error>
): Promise<T | Error> {
	// Both sources signal failure by resolving to `Error`. Anything else — a rejected fetch,
	// or `getBackendProxyUrl` throwing for want of a workspace — would otherwise skip the
	// second attempt entirely and propagate out of type acquisition.
	const settled = async (source: () => Promise<T | Error>) => {
		try {
			return await source()
		} catch (e) {
			return new Error(`Type acquisition request failed: ${e}`)
		}
	}

	const [first, second] = (await isRegistryConfigured()) ? [viaProxy, viaCdn] : [viaCdn, viaProxy]
	const result = await settled(first)
	return result instanceof Error ? await settled(second) : result
}

export const getNPMVersionsForModule = (moduleName: string, resLimit: ResLimit) =>
	fromPreferredSource<{ tags: Record<string, string>; versions: string[] }>(
		() =>
			api(`https://data.jsdelivr.com/v1/package/npm/${moduleName}`, resLimit, {
				cache: 'no-store'
			}),
		() => backendProxyApi(`/metadata/${encodeURIComponent(moduleName)}`, resLimit)
	)

export const getNPMVersionForModuleReference = (
	moduleName: string,
	reference: string,
	resLimit: ResLimit
) =>
	fromPreferredSource<{ version: string | null }>(
		() =>
			api(`https://data.jsdelivr.com/v1/package/resolve/npm/${moduleName}@${reference}`, resLimit),
		() =>
			backendProxyApi(
				`/resolve/${encodeURIComponent(moduleName)}?tag=${encodeURIComponent(reference)}`,
				resLimit
			)
	)

export type NPMTreeMeta = {
	default: string
	files: Array<{ name: string }>
	moduleName: string
	version: string
	raw: string
}

export const getFiletreeForModuleWithVersion = async (
	moduleName: string,
	version: string,
	raw: string,
	resLimit: ResLimit
) => {
	const res = await fromPreferredSource<NPMTreeMeta>(
		() => api(`https://data.jsdelivr.com/v1/package/npm/${moduleName}@${version}/flat`, resLimit),
		() =>
			backendProxyApi(
				`/filetree/${encodeURIComponent(moduleName)}/${encodeURIComponent(version)}`,
				resLimit
			)
	)
	return res instanceof Error ? res : { ...res, moduleName, version, raw }
}

export const getDTSFileForModuleWithVersion = (
	moduleName: string,
	version: string,
	// file comes with a prefix /
	file: string
) =>
	fromPreferredSource<string>(
		() => text(`https://cdn.jsdelivr.net/npm/${moduleName}@${version}${file}`),
		async () => {
			const proxyUrl = `${getBackendProxyUrl()}/file/${encodeURIComponent(moduleName)}/${encodeURIComponent(version)}${file}`
			const proxied = await text(proxyUrl, { credentials: 'include' })
			// The callers in ata/index.ts log a fixed message and drop the value, so a cause
			// left inside the returned `Error` is a cause nothing ever prints.
			if (proxied instanceof Error) console.warn('Backend proxy failed for file', proxied)
			return proxied
		}
	)

/**
 * Reading the body can fail as readily as connecting, and both have to surface as a value
 * rather than a rejection for the caller's fallback to run.
 */
async function text(url: string, init?: RequestInit): Promise<string | Error> {
	try {
		const res = await limit(() => fetch(url, init))
		return res.ok ? await res.text() : new Error(`${res.status} for ${url}`)
	} catch (e) {
		return new Error(`Request to ${url} failed: ${e}`)
	}
}

export interface ResLimit {
	usage: number
}

export function isOverlimit(resLimit: ResLimit) {
	return resLimit.usage > 5000000
}

export const limit = pLimit(6)

function api<T>(url: string, resLimit: ResLimit, init?: RequestInit): Promise<T | Error> {
	if (isOverlimit(resLimit)) {
		console.warn(
			`Exceeded limit of types downloaded for the needs of the assistant fetching: ${url}, ${resLimit.usage}`
		)
		return Promise.resolve(new Error('Exceeded limit of 100MB of data downloaded.'))
	}

	// Every caller decides what to do next by testing the resolved value for `Error`, so a
	// rejection here is not an alternative signal: it skips the backend-proxy fallback and
	// propagates out of type acquisition entirely.
	return limit(() =>
		fetch(url, init)
			.then((res) => {
				if (res.ok) {
					return res.text().then((text) => {
						resLimit.usage += text.length
						console.log('resLimit', url, resLimit.usage)

						return JSON.parse(text) as T
					}) as Promise<T | Error>
				} else {
					return new Error('OK')
				}
			})
			.catch((e) => new Error(`Request to ${url} failed: ${e}`))
	)
}
