import type { FlowModule } from '$lib/gen'

export type EarlyReturnCandidate = {
	id: string
	// False for a step inside a branch to one: it only runs when its branch is taken.
	deterministic: boolean
}

// Inside a branch to one, these return a list of jobs, and the sync wait loop only resolves the
// success of a list from the top-level run's modules. A loop's inner steps run once per iteration
// and so are not offered either.
const NOT_NESTABLE = new Set(['branchall', 'forloopflow', 'whileloopflow'])

export function earlyReturnCandidates(modules: FlowModule[]): EarlyReturnCandidate[] {
	return modules.flatMap((module) => [
		{ id: module.id, deterministic: true },
		...insideBranchOne(module)
	])
}

function insideBranchOne(module: FlowModule): EarlyReturnCandidate[] {
	if (module.value.type !== 'branchone') return []
	return [module.value.default, ...module.value.branches.map((b) => b.modules)]
		.flat()
		.filter((child) => !NOT_NESTABLE.has(child.value.type))
		.flatMap((child) => [{ id: child.id, deterministic: false }, ...insideBranchOne(child)])
}
