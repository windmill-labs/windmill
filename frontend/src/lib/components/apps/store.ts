import { writable } from "svelte/store";
import type {
    App,

} from './types'

export const importStore = writable<App | undefined>(undefined)