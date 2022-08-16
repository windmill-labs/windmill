import { writable } from 'svelte/store'
import { scrollIntoView } from './utils'

export const stepOpened = writable<string | undefined>(undefined)
