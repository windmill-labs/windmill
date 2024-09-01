import { goto as svelteGoto } from '$app/navigation'
import { base as svelteBase } from '$app/paths'

export function goto(path, options = {}) {
	if (svelteBase == '' || path.startsWith('?')) {
		return svelteGoto(path, options)
	} else {
		const fullPath = path.startsWith(svelteBase) ? path : `${svelteBase}${path}`
		return svelteGoto(fullPath, options)
	}
}

export async function setQuery(
	url: URL,
	key: string,
	value: string | undefined,
	currentHash: string | undefined = undefined
): Promise<void> {
	if (value !== undefined) {
		url.searchParams.set(key, value)
	} else {
		url.searchParams.delete(key)
	}

	let searchParams = url.searchParams.toString()

	await goto(currentHash ? `?${searchParams}${currentHash}` : `?${searchParams}`)
}
