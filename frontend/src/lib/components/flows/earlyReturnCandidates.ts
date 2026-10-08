import type { FlowModule } from '$lib/gen'
import { defaultIfEmptyString } from '$lib/utils'

export type EarlyReturnCandidate = {
	id: string
	// Where a node inside a branch to one sits, e.g. `a › Branch 1`. Absent at top level.
	location?: string
	// False for a node inside a branch to one: it only runs when its branch is taken.
	deterministic: boolean
}

// Inside a branch to one, these return a list of jobs, and the sync wait loop only resolves the
// success of a list from the top-level run's modules. A loop's inner steps run once per iteration
// and so cannot be an early return node either.
const NOT_NESTABLE = new Set(['branchall', 'forloopflow', 'whileloopflow'])

export function earlyReturnCandidates(modules: FlowModule[]): EarlyReturnCandidate[] {
	const candidates: EarlyReturnCandidate[] = []
	for (const module of modules) {
		candidates.push({ id: module.id, deterministic: true })
		collectBranchOne(module, [], candidates)
	}
	return candidates
}

function collectBranchOne(
	module: FlowModule,
	path: string[],
	candidates: EarlyReturnCandidate[]
): void {
	if (module.value.type !== 'branchone') return
	// Labels match the ones the flow graph shows.
	const branches = [
		{ label: 'Default', modules: module.value.default },
		...module.value.branches.map((b, i) => ({
			label: defaultIfEmptyString(b.summary, 'Branch ' + (i + 1)),
			modules: b.modules
		}))
	]
	for (const branch of branches) {
		const location = [...path, module.id, branch.label]
		for (const child of branch.modules) {
			if (NOT_NESTABLE.has(child.value.type)) continue
			candidates.push({ id: child.id, location: location.join(' › '), deterministic: false })
			collectBranchOne(child, location, candidates)
		}
	}
}
