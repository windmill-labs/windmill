import type { ExtendedOpenFlow } from './types'
import type { FlowState } from './flowState'
import type { StateStore } from '$lib/utils'
import type { History } from '$lib/history.svelte'
import { push } from '$lib/history.svelte'
import { dfs } from './dfs'
import { findModuleInFlow } from './flowTree'
import { deleteFlowStateById } from './flowStateUtils.svelte'

type Ctx = {
	flowStore: StateStore<ExtendedOpenFlow>
	flowStateStore: StateStore<FlowState>
	history: History<ExtendedOpenFlow>
}

/** Append an empty branch to a branchone/branchall step. */
export function addBranch(moduleId: string, { flowStore, history }: Omit<Ctx, 'flowStateStore'>) {
	push(history, flowStore.val)
	const module = findModuleInFlow(flowStore.val.value, moduleId)
	if (!module) throw new Error(`Node ${moduleId} not found`)

	if (module.value.type === 'branchone' || module.value.type === 'branchall') {
		module.value.branches.push({ summary: '', expr: 'false', modules: [] })
	}
}

/**
 * Drop a branch and the flow state of every step inside it.
 *
 * `index` counts the way the graph lays the branches out, where a branchone's default
 * occupies slot 0 — one ahead of the same branch's position in `value.branches`. Callers
 * working from the array (the settings panel) must add that offset back.
 */
export function removeBranch(
	moduleId: string,
	index: number,
	{ flowStore, flowStateStore, history }: Ctx
) {
	push(history, flowStore.val)
	const module = findModuleInFlow(flowStore.val.value, moduleId)
	if (!module) throw new Error(`Node ${moduleId} not found`)

	if (module.value.type === 'branchone' || module.value.type === 'branchall') {
		const offset = module.value.type === 'branchone' ? 1 : 0
		const at = index - offset

		if (module.value.branches[at]?.modules) {
			const leaves = dfs(module.value.branches[at].modules, (mod) => mod.id)
			leaves.forEach((leafId: string) => deleteFlowStateById(leafId, flowStateStore))
		}

		module.value.branches.splice(at, 1)
	}
}

/** Slot a branch occupies in the graph's numbering, from its index in `value.branches`. */
export function graphBranchIndex(type: 'branchone' | 'branchall', arrayIndex: number): number {
	return type === 'branchone' ? arrayIndex + 1 : arrayIndex
}
