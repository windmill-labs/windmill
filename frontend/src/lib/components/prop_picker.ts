import type { Writable } from 'svelte/store'
import type { PickableProperties } from '$lib/components/flows/previousResults'

export type FlowPropPickerConfig = {
	clearFocus: () => void
	onSelect: (path: string) => boolean
}

export type PropPickerContext = {
	flowPropPickerConfig: Writable<FlowPropPickerConfig | undefined>
	pickablePropertiesFiltered: Writable<PickableProperties | undefined>
	/** When it returns true, PropPickerWrapper keeps the prop picker collapsed and
	 *  only slides it in while a "connect" is active (used by the sessions modal
	 *  panel, where horizontal space is tight). Omitted/false → always shown. */
	collapsePropPickerUntilConnect?: () => boolean
}
