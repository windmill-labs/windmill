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

/**
 * The deployed counterpart of a step, matched by id rather than by position. Takes either
 * one deployed step to search under, or the deployed flow's top-level module list.
 *
 * The saved flow keeps the order it was deployed in, so any structural edit since —
 * reordering branches, inserting or deleting a step — shifts positions apart. Matching by
 * index would then pair a step with a different step's deployed code, which surfaces as
 * the wrong "last deployed" in the diff.
 */
export function savedModuleById(
	saved: FlowModule | FlowModule[] | undefined,
	id: string | undefined
): FlowModule | undefined {
	if (!saved || !id) return undefined
	let found: FlowModule | undefined = undefined
	const walk = (modules: FlowModule[]) => {
		for (const m of modules) {
			if (found) return
			if (m.id === id) {
				found = m
				return
			}
			const v = m.value
			if (v.type === 'forloopflow' || v.type === 'whileloopflow') walk(v.modules)
			else if (v.type === 'branchone') {
				walk(v.default)
				for (const b of v.branches) walk(b.modules)
			} else if (v.type === 'branchall') {
				for (const b of v.branches) walk(b.modules)
			}
		}
	}
	walk(Array.isArray(saved) ? saved : [saved])
	return found
}
