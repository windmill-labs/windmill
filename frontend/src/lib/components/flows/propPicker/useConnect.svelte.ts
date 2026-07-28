import type { Writable } from 'svelte/store'
import type { FlowPropPickerConfig } from '$lib/components/prop_picker'
import { graphParticipates, nextArmed, type ArmedTarget } from './connectPolicy'

/**
 * Holds the panel's single armed connect target and keeps the graph in step with it.
 *
 * The graph is driven imperatively from `arm`/`disarm` rather than from an `$effect`:
 * an effect that both reads the armed state and writes the shared store is the shape that
 * self-invalidates.
 */
export function useConnect(opts: {
	inModalPanel: () => boolean
	hasPickableProperties: () => boolean
	flowPropPickerConfig: Writable<FlowPropPickerConfig | undefined>
}) {
	let armed = $state<ArmedTarget | undefined>(undefined)

	function syncGraph() {
		const participates = graphParticipates(armed, {
			inModalPanel: opts.inModalPanel(),
			hasPickableProperties: opts.hasPickableProperties()
		})
		opts.flowPropPickerConfig.set(
			participates
				? {
						onSelect: (path: string) => {
							resolve(path)
							return true
						},
						clearFocus: disarm
					}
				: undefined
		)
	}

	function arm(target: ArmedTarget) {
		armed = nextArmed(armed, target)
		syncGraph()
	}

	function disarm() {
		armed = undefined
		opts.flowPropPickerConfig.set(undefined)
	}

	/** Deliver a picked property. Disarms first, so a target can never be written twice. */
	function resolve(path: string) {
		const target = armed
		disarm()
		target?.onSelect(path)
	}

	return {
		get armed() {
			return armed
		},
		isArmed: (id: string) => armed?.id === id,
		arm,
		disarm,
		resolve
	}
}
