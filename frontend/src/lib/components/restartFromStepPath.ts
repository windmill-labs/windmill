import type { FlowModule } from '$lib/gen'

export type ContainerType = 'branchone' | 'forloopflow' | 'flow' | 'whileloopflow' | 'branchall'

export type AncestorEntry = {
	stepId: string
	type: ContainerType
	/** For BranchOne: -1 means default branch, 0..N-1 means branches[i] */
	branchIndex?: number
	/** True for parallel ForLoop / BranchAll — backend rejects nested restart
	 * inside parallel containers, so the caller should hide the restart button. */
	parallel?: boolean
}

export type StepPath = {
	target: FlowModule
	ancestors: AncestorEntry[]
}

/**
 * Walks the flow value tree to locate `targetId`. Returns the target module and
 * the chain of containers from the root down to (but not including) the target.
 * Returns `undefined` if the step is not found in this flow value.
 *
 * Subflow boundaries are NOT crossed: if the target sits inside a `Flow{path}`
 * step's referenced flow, this returns `undefined` because we don't have that
 * subflow's value here.
 */
export function findStepPath(modules: FlowModule[], targetId: string): StepPath | undefined {
	for (const mod of modules) {
		if (mod.id === targetId) {
			return { target: mod, ancestors: [] }
		}
		const value = mod.value
		if (value.type === 'forloopflow' || value.type === 'whileloopflow') {
			const sub = findStepPath(value.modules, targetId)
			if (sub) {
				return {
					target: sub.target,
					ancestors: [
						{ stepId: mod.id, type: value.type, parallel: value.parallel === true },
						...sub.ancestors
					]
				}
			}
		} else if (value.type === 'branchone') {
			const allBranches: { idx: number; modules: FlowModule[] }[] = [
				{ idx: -1, modules: value.default }
			]
			value.branches.forEach((b, i) => allBranches.push({ idx: i, modules: b.modules }))
			for (const { idx, modules: bm } of allBranches) {
				const sub = findStepPath(bm, targetId)
				if (sub) {
					return {
						target: sub.target,
						ancestors: [{ stepId: mod.id, type: 'branchone', branchIndex: idx }, ...sub.ancestors]
					}
				}
			}
		} else if (value.type === 'branchall') {
			for (let i = 0; i < value.branches.length; i++) {
				const sub = findStepPath(value.branches[i].modules, targetId)
				if (sub) {
					return {
						target: sub.target,
						ancestors: [
							{
								stepId: mod.id,
								type: 'branchall',
								branchIndex: i,
								parallel: value.parallel === true
							},
							...sub.ancestors
						]
					}
				}
			}
		}
	}
	return undefined
}

/**
 * Inline-expanded subflows produce step IDs like
 * `subflow:<outer_subflow_step>:[<nested_subflow_step>:...]<leaf>`. Each `:`-separated
 * segment after the `subflow:` marker is a step ID; the last segment is the leaf
 * (the user's selected step) and the preceding segments are subflow steps along
 * the way (each one a `Flow{path}` module).
 *
 * Returns the parsed segments + leaf, or `undefined` if `id` is not a subflow-prefixed
 * step.
 */
export function parseExpandedSubflowId(
	id: string
): { subflowSteps: string[]; leaf: string } | undefined {
	if (!id.startsWith('subflow:')) {
		return undefined
	}
	const parts = id
		.slice('subflow:'.length)
		.split(':')
		.filter((p) => p.length > 0)
	if (parts.length < 2) {
		return undefined
	}
	const leaf = parts[parts.length - 1]
	const subflowSteps = parts.slice(0, -1)
	return { subflowSteps, leaf }
}
