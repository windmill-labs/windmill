import { writable } from 'svelte/store'

const store = writable<IsOpenStoreItem>({})

export type IsOpenStoreItem = Record<string, boolean>

export const isOpenStore = {
	subscribe: store.subscribe,
	update: store.update,
	set: store.set,
	/** If an item is already set, it won't get updated. */
	addItems: (items: IsOpenStoreItem[]) => {
		let newItems = {}
		items.forEach((item) => (newItems = { ...newItems, ...item }))
		store.update((last) => ({ ...newItems, ...last }))
	},
	toggle: (id: string) => {
		store.update((last) => ({ ...last, [id]: !last[id] }))
	},
	reset: () => store.set({})
}
