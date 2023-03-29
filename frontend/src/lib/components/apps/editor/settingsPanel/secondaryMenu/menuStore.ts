import type { SvelteComponent } from 'svelte'
import { writable } from 'svelte/store'

export const SECONDARY_MENU_ID = 'app-secondary-menu' as const

export interface SecondaryMenuStore {
	isOpen: boolean
	component?: typeof SvelteComponent | string
	props: Record<string, any>
	onClose?: (() => void) | undefined
}

const store = writable<SecondaryMenuStore>({ isOpen: false, component: undefined, props: {} })

export const secondaryMenu = {
	subscribe: store.subscribe,
	open: (
		component: SecondaryMenuStore['component'],
		props: SecondaryMenuStore['props'] = {},
		onClose: (() => void) | undefined = undefined
	) => {
		store.set({ isOpen: true, component, props, onClose })
	},
	close: () => {
		store.update((state) => {
			if (state.onClose) state.onClose()
			return { isOpen: false, component: undefined, props: {} }
		})
	}
} as const
