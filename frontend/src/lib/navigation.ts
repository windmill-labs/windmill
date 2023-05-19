import { goto } from '$app/navigation'

export async function setQuery(url: URL, key: string, value: string): Promise<void> {
	url.searchParams.set(key, value)
	await goto(`?${url.searchParams.toString()}`)
}
