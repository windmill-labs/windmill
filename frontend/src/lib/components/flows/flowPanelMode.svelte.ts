import { resolvePanelMode, type FlowPanelMode, type FlowPanelPreference } from './panelPlacement'

/**
 * Holds the step panel's placement preference and the editor's measured width, and reads
 * the resolution off `resolvePanelMode`. The preference is not persisted: it lasts as long
 * as the editor is open, so every flow opens on `auto` and a pin is a deliberate act each
 * time.
 */
export function useFlowPanelMode(opts: { enabled: () => boolean }) {
	let preference = $state<FlowPanelPreference>('auto')
	let width = $state(0)

	return {
		get preference(): FlowPanelPreference {
			return preference
		},
		set preference(next: FlowPanelPreference) {
			preference = next
		},
		get mode(): FlowPanelMode {
			return resolvePanelMode({ enabled: opts.enabled(), preference, width })
		},
		/**
		 * Whether `mode` reflects a real layout. Until the first measurement lands, `mode` is
		 * `docked` because that is the safe thing to render, not because the editor is wide.
		 */
		get measured(): boolean {
			return width > 0
		},
		/** Fed by the editor root's measured width; drives `auto` in both directions. */
		measure(measured: number | null | undefined) {
			width = measured ?? 0
		}
	}
}
