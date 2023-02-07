import { readable } from 'svelte/store'

export async function goto(x) {
	console.error('Goto not implemented')
}

export const browser = true
export const dev = true

export const page = readable({ url: document.URL }, function start(set) {
	return function stop() {}
})
