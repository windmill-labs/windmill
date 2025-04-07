import { writable } from 'svelte/store'

export const LAYOUT_CONTEXT_KEY = 'layoutContext'

export interface LayoutContext {
	topBarHeight: number
}

export const layoutStore = writable<LayoutContext>({
	topBarHeight: 0
})

export const setTopBarHeight = (height: number) => {
	layoutStore.update((state) => ({ ...state, topBarHeight: height }))
}
