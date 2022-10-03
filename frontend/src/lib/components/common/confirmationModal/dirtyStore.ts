import { writable } from 'svelte/store'

export const dirtyStore = writable<boolean>(false)
