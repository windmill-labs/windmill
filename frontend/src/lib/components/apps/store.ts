import type { Policy } from "$lib/gen";
import { writable } from "svelte/store";
import type {
    App,

} from './types'

export const importStore = writable<{ summary: string, value: App, policy: Policy } | undefined>(undefined)