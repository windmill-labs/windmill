import type { FlowModule } from '$lib/gen'

/**
 * A read/write slot for one step, anchored by id rather than by array position. Editors
 * write through their binding as they unmount, and a delete has already spliced the array
 * by then — by index that write would hit whichever step took the deleted one's place.
 *
 * Svelte extracts the getter once per block instance, so the anchor cannot drift; the
 * hosting `{#each}` must therefore be keyed by id, or a reused block keeps a dead anchor.
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
