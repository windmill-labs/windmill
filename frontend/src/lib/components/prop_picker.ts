import type { Writable } from 'svelte/store'
import type { PickableProperties } from '$lib/components/flows/previousResults'

export type FlowPropPickerConfig = {
	clearFocus: () => void
	onSelect: (path: string) => boolean
}

export type PropPickerContext = {
	flowPropPickerConfig: Writable<FlowPropPickerConfig | undefined>
	pickablePropertiesFiltered: Writable<PickableProperties | undefined>
	/** True when the panel is a modal, which covers the graph. Connecting there could never
	 *  be completed by clicking a step node, so the graph stays out of it. */
	inModalPanel?: () => boolean
}
