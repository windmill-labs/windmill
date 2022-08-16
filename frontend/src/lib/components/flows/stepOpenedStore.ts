import { writable } from 'svelte/store'
import { scrollIntoView } from './utils'

export const stepOpened = writable<string | undefined>(undefined)

stepOpened.subscribe((insertAt) => {
	setTimeout(() => scrollIntoView(document.querySelector(`#module-${insertAt}`)), 100)
})
