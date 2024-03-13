//  https://github.com/jsdelivr/data.jsdelivr.com

export const getNPMVersionsForModule = (moduleName: string, resLimit: ResLimit) => {
	const url = `https://data.jsdelivr.com/v1/package/npm/${moduleName}`
	return api<{ tags: Record<string, string>; versions: string[] }>(url, resLimit, {
		cache: 'no-store'
	})
}

export const getNPMVersionForModuleReference = (
	moduleName: string,
	reference: string,
	resLimit: ResLimit
) => {
	const url = `https://data.jsdelivr.com/v1/package/resolve/npm/${moduleName}@${reference}`
	return api<{ version: string | null }>(url, resLimit)
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

export interface ResLimit {
	usage: number
}

export function isOverlimit(resLimit: ResLimit) {
	return resLimit.usage > 5000000
}

function api<T>(url: string, resLimit: ResLimit, init?: RequestInit): Promise<T | Error> {
	if (isOverlimit(resLimit)) {
		console.warn(
			`Exceeded limit of types downloaded for the needs of the assistant fetching: ${url}, ${resLimit.usage}`
		)
		return new Promise(() => new Error('Exceeded limit of 100MB of data downloaded.'))
	}

	return fetch(url, init).then((res) => {
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
}
