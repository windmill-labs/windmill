import type { FlowModule } from '$lib/gen'

/**
 * A read/write slot for one step, anchored by id rather than by array position.
 *
 * Panels bind to a step for as long as they are mounted, and their editors write through
 * that binding when they unmount — including the trailing keystrokes flushed on destroy.
 * Deleting a step splices the array, so an index-anchored binding would resolve to
 * whichever step took the deleted one's place and overwrite it. Anchoring by id makes a
 * write to a departed step land on the detached object instead, where it is harmless.
 */
export function moduleSlot(
	getModules: () => FlowModule[],
	id: string | undefined,
	detached: FlowModule
) {
	return {
		get: (): FlowModule => getModules().find((m) => m.id === id) ?? detached,
		set: (v: FlowModule) => {
			const modules = getModules()
			const i = modules.findIndex((m) => m.id === id)
			if (i !== -1) {
				modules[i] = v
			}
		}
	}
}
