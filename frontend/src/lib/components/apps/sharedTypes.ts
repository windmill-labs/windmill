import type { Schema } from '$lib/common'
import type { Preview } from '$lib/gen'

export type InlineScript = {
	content: string
	language: Preview['language'] | 'frontend'
	path?: string
	schema?: Schema
	lock?: string
	cache_ttl?: number
	refreshOn?: { id: string; key: string }[]
	suggestedRefreshOn?: { id: string; key: string }[]
	id?: number
}
