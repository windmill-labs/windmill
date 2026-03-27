import { goto as svelteGoto } from '$app/navigation'
import { base as svelteBase } from '$app/paths'
import { toWorkspacePath } from './workspaceUrl'

export function goto(path: string, options = {}) {
	if (path.startsWith('?') || path.startsWith('#')) {
		return svelteGoto(path, options)
	}

	let fullPath = toWorkspacePath(path)

	if (svelteBase && !fullPath.startsWith(svelteBase)) {
		fullPath = `${svelteBase}${fullPath}`
	}

	return svelteGoto(fullPath, options)
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
