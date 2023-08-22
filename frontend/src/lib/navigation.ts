import { goto } from '$app/navigation'

export async function setQuery(
	url: URL,
	key: string,
	value: string | undefined,
	keepHash: boolean = false
): Promise<void> {
	const currentHash = url.hash

	if (value !== undefined) {
		url.searchParams.set(key, value)
	} else {
		url.searchParams.delete(key)
	}

	await goto(
		keepHash && currentHash
			? `?${url.searchParams.toString()}${currentHash}`
			: `?${url.searchParams.toString()}`
	)
}
