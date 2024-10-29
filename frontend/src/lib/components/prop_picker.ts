import type { Writable } from 'svelte/store'

type InsertionMode = 'append' | 'connect' | 'insert'

type SelectCallback = (path: string) => boolean

export type PropPickerConfig = {
	insertionMode: InsertionMode
	propName: string
	onSelect: SelectCallback
}

export type PropPickerWrapperContext = {
	propPickerConfig: Writable<PropPickerConfig | undefined>
	focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => void
	clearFocus: () => void
}
