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
	/** Mirrors the armed target for an in-panel picker, which filters by its prop name. */
	localConfig?: Writable<
		{ propName?: string; onSelect: (path: string) => boolean; clearFocus: () => void } | undefined
	>
	/** Anonymous counters for how often an armed connect ends in a pick. */
	onEvent?: (event: 'open' | 'insert' | 'abandon') => void
}) {
	let armed = $state<ArmedTarget | undefined>(undefined)

	function sync() {
		opts.localConfig?.set(
			armed
				? {
						propName: armed.id,
						onSelect: (path: string) => {
							resolve(path)
							return true
						},
						clearFocus: disarm
					}
				: undefined
		)
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
		const next = nextArmed(armed, target)
		// Arming is exclusive, so anything already armed ends here — and clicking the armed
		// input again disarms it, ending a target without opening another.
		if (armed) opts.onEvent?.('abandon')
		if (next) opts.onEvent?.('open')
		armed = next
		sync()
	}

	function clear() {
		armed = undefined
		opts.localConfig?.set(undefined)
		opts.flowPropPickerConfig.set(undefined)
	}

	function disarm() {
		// `flowPropPickerConfig` is shared by every connect surface in the panel, so a component
		// that holds nothing must not clear it — otherwise closing any picker cancels whichever
		// input is actually armed, and the pick that follows has nowhere to land.
		if (!armed) return
		opts.onEvent?.('abandon')
		clear()
	}

	/** Deliver a picked property. Disarms first, so a target can never be written twice. */
	function resolve(path: string) {
		const target = armed
		if (!target) return
		opts.onEvent?.('insert')
		clear()
		target.onSelect(path)
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
