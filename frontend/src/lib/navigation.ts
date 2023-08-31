import { goto } from '$app/navigation'

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
