import { goto as svelteGoto } from '$app/navigation'
import { base } from '$lib/navigation'

export { base } from '$app/paths'

export function goto(path, options = {}) {
	const fullPath = path.startsWith(base) ? path : `${base}${path}`;
	return svelteGoto(fullPath, options);
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

	await goto(
		currentHash
			? `?${url.searchParams.toString()}${currentHash}`
			: `?${url.searchParams.toString()}`
	)
}
