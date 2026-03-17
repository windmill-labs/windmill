import type { NewScript } from '$lib/gen'
import { writable } from 'svelte/store'

export const importScriptStore = writable<NewScript | undefined>(undefined)
