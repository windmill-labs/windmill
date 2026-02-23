import type { Writable } from 'svelte/store'
import type { PickableProperties } from '$lib/components/flows/previousResults'

export type FlowPropPickerConfig = {
	clearFocus: () => void
	onSelect: (path: string) => boolean
}

export type PropPickerContext = {
	flowPropPickerConfig: Writable<FlowPropPickerConfig | undefined>
	pickablePropertiesFiltered: Writable<PickableProperties | undefined>
}
