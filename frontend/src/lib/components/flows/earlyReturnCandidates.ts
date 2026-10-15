import type { FlowModule } from '$lib/gen'

export type EarlyReturnCandidate = {
	id: string
	// False for a step inside a branch to one: it only runs when its branch is taken.
	deterministic: boolean
}

// A loop's inner steps run once per iteration, so only the loop itself is offered.
export function earlyReturnCandidates(modules: FlowModule[]): EarlyReturnCandidate[] {
	return withinBranches(modules, true)
}

function withinBranches(modules: FlowModule[], deterministic: boolean): EarlyReturnCandidate[] {
	return modules.flatMap((module) => {
		const value = module.value
		const nested =
			value.type === 'branchone'
				? withinBranches([value.default, ...value.branches.map((b) => b.modules)].flat(), false)
				: value.type === 'branchall'
					? withinBranches(
							value.branches.flatMap((b) => b.modules),
							deterministic
						)
					: []
		return [{ id: module.id, deterministic }, ...nested]
	})
}
