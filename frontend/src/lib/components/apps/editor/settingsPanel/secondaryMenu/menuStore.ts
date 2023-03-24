import type { SvelteComponent } from 'svelte'
import { writable } from 'svelte/store'

export const SECONDARY_MENU_ID = 'app-secondary-menu' as const

export interface SecondaryMenuStore {
	isOpen: boolean
	component?: typeof SvelteComponent | string
	props: Record<string, any>
}

const store = writable<SecondaryMenuStore>({ isOpen: false, component: undefined, props: {} })

export const secondaryMenu = {
	subscribe: store.subscribe,
	open: (component: SecondaryMenuStore['component'], props: SecondaryMenuStore['props'] = {}) => {
		store.set({ isOpen: true, component, props })
	},
	close: () => store.set({ isOpen: false, component: undefined, props: {} })
} as const
