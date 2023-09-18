import { writable, type Writable } from 'svelte/store'

export interface SecondaryMenuStore {
	isOpen: boolean
	component?: any | string
	props: Record<string, any>
	onClose?: (() => void) | undefined
}

export const secondaryMenuRightStore = writable<SecondaryMenuStore>({
	isOpen: false,
	component: undefined,
	props: {}
})
export const secondaryMenuLeftStore = writable<SecondaryMenuStore>({
	isOpen: false,
	component: undefined,
	props: {}
})

export const secondaryMenuRight = secondaryMenuController(secondaryMenuRightStore)
export const secondaryMenuLeft = secondaryMenuController(secondaryMenuLeftStore)

export function secondaryMenuController(store: Writable<SecondaryMenuStore>) {
	return {
		subscribe: store.subscribe,
		toggle: (
			component: SecondaryMenuStore['component'],
			props: SecondaryMenuStore['props'] = {},
			onClose: (() => void) | undefined = undefined
		) => {
			store.update((str) => ({ isOpen: !str.isOpen, component, props, onClose }))
		},
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
}
