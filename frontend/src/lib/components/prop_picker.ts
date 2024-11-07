import type { Writable } from 'svelte/store'
import type { PickableProperties } from '$lib/components/flows/previousResults'

type InsertionMode = 'append' | 'connect' | 'insert'

export type FlowPropPickerConfig = {
	insertionMode: InsertionMode
	clearFocus: () => void
	onSelect: (path: string) => boolean
}

export type PropPickerContext = {
	flowPropPickerConfig: Writable<FlowPropPickerConfig | undefined>
	pickablePropertiesFiltered: Writable<PickableProperties | undefined>
}
