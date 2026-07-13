import type { FlowModule } from '$lib/gen'

export type NestedRestartStep = { step_id: string; branch_or_iteration_n?: number }

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

/** Per-module display state the graph tracks; only the ForLoop bits matter here. */
export type ForloopGraphState = { selectedForloopIndex?: number; flow_jobs?: unknown[] }
/** Minimal shape of a top-level `flow_status.modules` entry we need. */
export type FlowStatusModuleLite = {
	id?: string
	branch_chosen?: { type: 'branch' | 'default'; branch?: number }
}

export type NestedRestartResult = {
	topStepId: string
	topBranchOrIterationN: number | undefined
	path: NestedRestartStep[]
	/** ForLoop iteration counts keyed by the popup's field key: `'top'` for the
	 * outer container, `inner-${n}` for `path[n]`. */
	iterationCounts: Record<string, number>
}

/**
 * True when the BranchOne ancestor's path branch (encoded as `branchIndex`,
 * where -1 is the default branch and 0..N-1 is `branches[i]`) is the branch the
 * original run actually took. If the user clicked a step inside a branch the run
 * didn't take, that step never executed and a restart there is impossible.
 *
 * `undefined` status means the BranchOne isn't at the level whose `flow_status`
 * we have (it lives on a child job we don't fetch) — permissive fallback: let
 * the backend reject if needed.
 */
function branchOneMatchesOriginal(
	flowStatusModules: FlowStatusModuleLite[] | undefined,
	stepId: string,
	branchIndex: number | undefined
): boolean {
	const status = flowStatusModules?.find((m) => m.id === stepId)
	const chosen = status?.branch_chosen
	if (!chosen) return status === undefined
	const taken = chosen.type === 'default' ? -1 : (chosen.branch ?? -1)
	return branchIndex === taken
}

/**
 * Build the restart chain from a top-level container of the running flow down to
 * the selected leaf, flattening BOTH kinds of nesting into one list:
 *   - containers within a flow value (sequential ForLoop / BranchOne), recovered
 *     by walking each level's modules with `findStepPath`
 *   - subflow boundaries (`Flow{path}` steps), encoded in the graph node id as
 *     `subflow:A:B:leaf`
 *
 * The graph node id records ONLY subflow boundaries, so a ForLoop/BranchOne
 * sitting above or between them is invisible to `parseExpandedSubflowId`; walking
 * each level recovers it. Without this, a subflow nested inside a ForLoop yields
 * an outer subflow step that isn't a top-level module, and the restart button
 * never appeared.
 *
 * Returns `null` when the step is top-level (caller handles that separately), the
 * leaf can't be located, or an unsupported container (parallel ForLoop/BranchAll,
 * WhileLoop, or a BranchOne branch the run didn't take) sits on the path.
 */
export function buildNestedRestartPath(opts: {
	selectedJobStep: string
	rawFlowModules: FlowModule[]
	flowStatusModules: FlowStatusModuleLite[] | undefined
	graphModuleStates: Record<string, ForloopGraphState>
	expandedSubflows: Record<string, { modules: FlowModule[] }>
}): NestedRestartResult | null {
	const {
		selectedJobStep,
		rawFlowModules,
		flowStatusModules,
		graphModuleStates,
		expandedSubflows
	} = opts

	const parse = parseExpandedSubflowId(selectedJobStep)
	const boundaries = parse?.subflowSteps ?? []
	const leaf = parse?.leaf ?? selectedJobStep

	// Graph-state key prefix for level `i`: '' at the running flow, then
	// `subflow:<boundaries[0..i-1]>:` once inside subflow boundary `i-1`. Only
	// subflow boundaries contribute to the prefix (ForLoop/BranchOne don't),
	// matching how the graph builds node ids.
	const graphPrefixFor = (i: number): string =>
		i === 0 ? '' : 'subflow:' + boundaries.slice(0, i).join(':') + ':'
	// Graph node id of subflow boundary `i` (also its `expandedSubflows` key).
	const boundaryNodeId = (i: number): string => graphPrefixFor(i) + boundaries[i]

	// One entry per container/subflow step from the top-level container down to
	// the leaf. `chain[0]` becomes the top-level restart step; the rest is the
	// nested path. `counts` maps a chain index to that ForLoop's recorded
	// iteration count.
	const chain: NestedRestartStep[] = []
	const counts: Record<number, number> = {}
	const appendStep = (stepId: string, isForloop: boolean, graphPrefix: string) => {
		const entry: NestedRestartStep = { step_id: stepId }
		if (isForloop) {
			// Default to the user's currently-open iteration; the popup surfaces
			// every value for confirmation/editing before submit.
			const key = graphPrefix + stepId
			entry.branch_or_iteration_n = graphModuleStates[key]?.selectedForloopIndex ?? 0
			counts[chain.length] = graphModuleStates[key]?.flow_jobs?.length ?? 0
		}
		chain.push(entry)
	}

	for (let i = 0; i <= boundaries.length; i++) {
		const isLeafLevel = i === boundaries.length
		const target = isLeafLevel ? leaf : boundaries[i]
		const graphPrefix = graphPrefixFor(i)
		// Level modules: the running flow at level 0, else the cached modules of
		// the subflow boundary we descended through.
		const modules = i === 0 ? rawFlowModules : expandedSubflows[boundaryNodeId(i - 1)]?.modules
		if (!modules) {
			// Subflow modules not loaded yet (leaf clicked before the subflow was
			// expanded): append the remaining boundaries and the leaf as a flat
			// best-effort continuation so the button still shows.
			for (let j = i; j < boundaries.length; j++) chain.push({ step_id: boundaries[j] })
			chain.push({ step_id: leaf })
			break
		}
		const path = findStepPath(modules, target)
		if (!path) {
			// Target missing from these (loaded) modules — a stale/inconsistent
			// cache. Same flat best-effort continuation as the not-loaded case:
			// remaining boundaries then the leaf, so the chain always ends at the
			// real leaf (never a subflow boundary) and the backend can validate.
			for (let j = i; j < boundaries.length; j++) chain.push({ step_id: boundaries[j] })
			chain.push({ step_id: leaf })
			break
		}
		// Gate on unsupported containers along the way. BranchOne branch-mismatch
		// is only checkable at the running flow's own level, where we have its
		// `flow_status`; deeper levels live on child jobs we don't fetch here, so
		// stay permissive and let the backend reject a branch the run didn't take.
		const blocked = path.ancestors.some(
			(a) =>
				a.type === 'branchall' ||
				a.type === 'whileloopflow' ||
				a.parallel === true ||
				(a.type === 'branchone' &&
					i === 0 &&
					!branchOneMatchesOriginal(flowStatusModules, a.stepId, a.branchIndex))
		)
		if (blocked) return null
		for (const a of path.ancestors) {
			appendStep(a.stepId, a.type === 'forloopflow', graphPrefix)
		}
		// Subflow boundaries are `Flow{path}` steps (no iteration); only the leaf
		// can itself be a ForLoop the user wants to restart at an iteration of.
		appendStep(target, isLeafLevel && path.target.value.type === 'forloopflow', graphPrefix)
	}

	// Need at least a top container plus the leaf; a lone entry means the leaf is
	// effectively top-level (handled by the caller's top-level path).
	if (chain.length < 2) return null

	const [top, ...rest] = chain
	// Re-key iteration counts to the popup's field keys: chain index 0 → 'top',
	// index k → 'inner-(k-1)' (matching `path[k-1]`).
	const iterationCounts: Record<string, number> = {}
	for (const [idxStr, n] of Object.entries(counts)) {
		const idx = Number(idxStr)
		iterationCounts[idx === 0 ? 'top' : `inner-${idx - 1}`] = n
	}
	return {
		topStepId: top.step_id,
		topBranchOrIterationN: top.branch_or_iteration_n,
		path: rest,
		iterationCounts
	}
}
