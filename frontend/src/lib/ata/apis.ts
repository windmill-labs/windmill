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

		return limit(() =>
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
		return new Error('Backend proxy not available')
	}
}

export const getNPMVersionsForModule = async (moduleName: string, resLimit: ResLimit) => {
	const url = `https://data.jsdelivr.com/v1/package/npm/${moduleName}`
	const result = await api<{ tags: Record<string, string>; versions: string[] }>(url, resLimit, {
		cache: 'no-store'
	})

	// If jsdelivr fails, try backend proxy
	if (result instanceof Error) {
		console.log('jsdelivr failed for', moduleName, 'trying backend proxy')
		return backendProxyApi<{ tags: Record<string, string>; versions: string[] }>(
			`/metadata/${encodeURIComponent(moduleName)}`,
			resLimit
		)
	}

	return result
}

export const getNPMVersionForModuleReference = async (
	moduleName: string,
	reference: string,
	resLimit: ResLimit
) => {
	const url = `https://data.jsdelivr.com/v1/package/resolve/npm/${moduleName}@${reference}`
	const result = await api<{ version: string | null }>(url, resLimit)

	// If jsdelivr fails, try backend proxy
	if (result instanceof Error) {
		console.log('jsdelivr failed for', moduleName, reference, 'trying backend proxy')
		return backendProxyApi<{ version: string | null }>(
			`/resolve/${encodeURIComponent(moduleName)}?tag=${encodeURIComponent(reference)}`,
			resLimit
		)
	}

	return result
}

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
	const url = `https://data.jsdelivr.com/v1/package/npm/${moduleName}@${version}/flat`
	const res = await api<NPMTreeMeta>(url, resLimit)
	if (res instanceof Error) {
		// Try backend proxy
		console.log('jsdelivr failed for', moduleName, version, 'trying backend proxy')
		const backendRes = await backendProxyApi<NPMTreeMeta>(
			`/filetree/${encodeURIComponent(moduleName)}/${encodeURIComponent(version)}`,
			resLimit
		)
		if (backendRes instanceof Error) {
			return res
		} else {
			return {
				...backendRes,
				moduleName,
				version,
				raw
			}
		}
	} else {
		return {
			...res,
			moduleName,
			version,
			raw
		}
	}
}

export const getDTSFileForModuleWithVersion = async (
	moduleName: string,
	version: string,
	file: string
) => {
	// file comes with a prefix /
	const url = `https://cdn.jsdelivr.net/npm/${moduleName}@${version}${file}`
	const res = await limit(() => fetch(url))
	if (res.ok) {
		return res.text()
	} else {
		// Try backend proxy
		console.log('jsdelivr failed for file', moduleName, version, file, 'trying backend proxy')
		try {
			const baseUrl = getBackendProxyUrl()
			const proxyUrl = `${baseUrl}/file/${encodeURIComponent(moduleName)}/${encodeURIComponent(version)}${file}`
			const proxyRes = await limit(() => fetch(proxyUrl, { credentials: 'include' }))
			if (proxyRes.ok) {
				return proxyRes.text()
			}
		} catch (e) {
			console.log('Backend proxy failed for file', e)
		}
		return new Error('OK')
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
		return new Promise(() => new Error('Exceeded limit of 100MB of data downloaded.'))
	}

	return limit(() =>
		fetch(url, init).then((res) => {
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
	)
}
