import { writable } from 'svelte/store'

export const ignoredTutorials = writable<number[]>([])
