import type { FlowModule } from '$lib/gen'

export type EarlyReturnCandidate = {
	id: string
	// False for a step inside a branch to one: it only runs when its branch is taken.
	deterministic: boolean
}

// Nested in a branch, these return a list of jobs, and the sync wait loop only resolves the
// success of a list from the top-level run's modules. A loop's inner steps run once per iteration
// and so are not offered either.
const NOT_NESTABLE = new Set(['branchall', 'forloopflow', 'whileloopflow'])

export function earlyReturnCandidates(modules: FlowModule[]): EarlyReturnCandidate[] {
	return modules.flatMap((module) => [
		{ id: module.id, deterministic: true },
		...insideBranches(module, true)
	])
}

function insideBranches(module: FlowModule, deterministic: boolean): EarlyReturnCandidate[] {
	const value = module.value
	let children: FlowModule[]
	if (value.type === 'branchone') {
		children = [value.default, ...value.branches.map((b) => b.modules)].flat()
		deterministic = false
	} else if (value.type === 'branchall') {
		children = value.branches.flatMap((b) => b.modules)
	} else {
		return []
	}
	return children.flatMap((child) => [
		...(NOT_NESTABLE.has(child.value.type) ? [] : [{ id: child.id, deterministic }]),
		...insideBranches(child, deterministic)
	])
}
