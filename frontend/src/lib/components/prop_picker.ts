import type { Writable } from 'svelte/store'
import type { PickableProperties } from '$lib/components/flows/previousResults'

type InsertionMode = 'append' | 'connect' | 'insert'

type SelectCallback = (path: string) => boolean

export type PropPickerConfig = {
	insertionMode: InsertionMode
	propName: string
	onSelect: SelectCallback
}

export type PropPickerWrapperContext = {
	propPickerConfig: Writable<PropPickerConfig | undefined>
	filteredPickableProperties: Writable<PickableProperties | undefined>
	inputMatches: Writable<{ word: string; value: string }[] | undefined>
	focusProp: (propName: string, insertionMode: InsertionMode, onSelect: SelectCallback) => void
	clearFocus: () => void
}
