import type { Policy } from '$lib/gen'
import { writable } from 'svelte/store'

export const importStore = writable<{ summary: string; value: any; policy: Policy } | undefined>(
	undefined
)
