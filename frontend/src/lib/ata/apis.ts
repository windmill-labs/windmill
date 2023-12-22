import type { ATABootstrapConfig } from './index'

//  https://github.com/jsdelivr/data.jsdelivr.com

export const getNPMVersionsForModule = (config: ATABootstrapConfig, moduleName: string) => {
	const url = `https://data.jsdelivr.com/v1/package/npm/${moduleName}`
	return api<{ tags: Record<string, string>; versions: string[] }>(config, url, {
		cache: 'no-store'
	})
}

export const getNPMVersionForModuleReference = (
	config: ATABootstrapConfig,
	moduleName: string,
	reference: string
) => {
	const url = `https://data.jsdelivr.com/v1/package/resolve/npm/${moduleName}@${reference}`
	return api<{ version: string | null }>(config, url)
}

export type NPMTreeMeta = {
	default: string
	files: Array<{ name: string }>
	moduleName: string
	version: string
	raw: string
}

export const getFiletreeForModuleWithVersion = async (
	config: ATABootstrapConfig,
	moduleName: string,
	version: string,
	raw: string
) => {
	const url = `https://data.jsdelivr.com/v1/package/npm/${moduleName}@${version}/flat`
	const res = await api<NPMTreeMeta>(config, url)
	if (res instanceof Error) {
		return res
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
	config: ATABootstrapConfig,
	moduleName: string,
	version: string,
	file: string
) => {
	// file comes with a prefix /
	const url = `https://cdn.jsdelivr.net/npm/${moduleName}@${version}${file}`
	const res = await fetch(url)
	if (res.ok) {
		return res.text()
	} else {
		return new Error('OK')
	}
}

function api<T>(config: ATABootstrapConfig, url: string, init?: RequestInit): Promise<T | Error> {
	return fetch(url, init).then((res) => {
		if (res.ok) {
			return res.json().then((f) => f as T)
		} else {
			return new Error('OK')
		}
	})
}
