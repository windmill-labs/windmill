import type { FlowModule, Job } from '$lib/gen'
import type { GraphModuleState } from './graph'
import { findStepPath, parseExpandedSubflowId, type AncestorEntry } from './restartFromStepPath'
import { emptyString } from '$lib/utils'

export type NestedRestartStep = { step_id: string; branch_or_iteration_n?: number }

/**
 * Returns true when the BranchOne ancestor's path branch (the one containing the
 * selected leaf, encoded as `branchIndex` where -1 means default and 0..N-1
 * means `branches[i]`) is the same branch the original run took. If the user
 * clicked a step inside an `else` branch the original run didn't take, the
 * step never executed — restart there is impossible.
 *
 * Three states for the lookup:
 *   - `status === undefined`: the BranchOne isn't at the parent's top level, so
 *     it lives on a child job we don't fetch here (e.g. nested inside a ForLoop
 *     iteration). Permissive fallback — let the backend reject if needed.
 *   - `status` defined but `chosen === undefined`: the BranchOne IS at the top
 *     level but has no chosen branch (never executed / skipped / still
 *     waiting). The leaf can't have run either, so reject.
 *   - both defined: compare against `branchIndex`.
 */
function branchOneAncestorMatchesOriginal(job: Job, ancestor: AncestorEntry): boolean {
	if (ancestor.type !== 'branchone') return true
	const status = job.flow_status?.modules?.find((m) => m.id === ancestor.stepId)
	const chosen = status?.branch_chosen
	if (!chosen) return status === undefined
	const taken = chosen.type === 'default' ? -1 : (chosen.branch ?? -1)
	return ancestor.branchIndex === taken
}

/**
 * Reactive state shared by the run page and the editor's flow preview to drive
 * the "restart flow at step" UI. Given the currently selected step, the
 * completed flow job, the graph's per-module display state, and any expanded
 * subflow definitions, derives:
 *
 *   - `selectedJobStepIsTopLevel` — whether the selected step appears in the
 *     parent flow's top-level `flow_status.modules`
 *   - `selectedJobStepType` — `single` / `forloop` / `branchall`, used by the
 *     popup to decide which controls to render for the OUTER container
 *   - `restartBranchNames` — for top-level BranchAll steps, the labels for the
 *     branch picker
 *   - `nestedRestartTopStepId` / `nestedRestartTopBranchOrIterationN` /
 *     `nestedRestartPath` — the API request shape when restarting at a nested
 *     step (single source of truth for both call sites)
 *   - `nestedRestartSupported` — gate for showing the button when the selected
 *     step is nested (subflow expansion or BranchOne / sequential ForLoop)
 *   - `topLevelLoopIteration` — when the selected step IS a top-level ForLoop,
 *     the iteration the user is currently viewing (pre-fills the popup input)
 *   - `iterationCounts` — map of step id → number of recorded iterations,
 *     so the popup can render an iteration `<select>` matching the graph's tabs
 *     instead of a free-form number input
 *
 * Inputs are passed as getters so the composable stays reactive in either
 * caller's signal graph.
 */
export function useNestedRestartState(opts: {
	selectedJobStep: () => string | undefined
	job: () => Job | undefined
	graphModuleStates: () => Record<string, GraphModuleState>
	/** Subflow definitions cached when the user expands a subflow in the graph,
	 * keyed by the graph node id (i.e. the prefixed `subflow:...:<step_id>` for
	 * nested subflows). Lets us walk inside subflows when building the nested
	 * restart path. */
	expandedSubflows?: () => Record<string, { modules: FlowModule[] }>
}) {
	let selectedJobStepIsTopLevel: boolean | undefined = $state(undefined)
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = $state('single')
	let restartBranchNames: [number, string][] = $state([])
	let nestedRestartTopStepId: string | undefined = $state(undefined)
	let nestedRestartTopBranchOrIterationN: number | undefined = $state(undefined)
	let nestedRestartPath: NestedRestartStep[] | undefined = $state(undefined)
	let nestedRestartSupported = $state(false)
	// Iteration counts keyed by the popup's field-key ('top' or 'inner-N').
	// Built during path construction so each entry uses the *correct* graph key
	// for its level (prefixed for in-subflow ancestors). Avoids the collision
	// that would happen if we double-indexed `iterationCounts` by bare step_id.
	let nestedPathIterationCounts: Record<string, number> = $state({})

	$effect(() => {
		const selectedJobStep = opts.selectedJobStep()
		const job = opts.job()
		const graphModuleStates = opts.graphModuleStates()
		const expandedSubflows = opts.expandedSubflows?.() ?? {}

		nestedRestartTopStepId = undefined
		nestedRestartTopBranchOrIterationN = undefined
		nestedRestartPath = undefined
		nestedRestartSupported = false
		restartBranchNames = []
		selectedJobStepIsTopLevel = undefined
		nestedPathIterationCounts = {}

		if (selectedJobStep === undefined || job?.flow_status?.modules === undefined) {
			return
		}

		const isTopLevel = job.flow_status.modules.findIndex((m) => m.id === selectedJobStep) >= 0
		selectedJobStepIsTopLevel = isTopLevel
		const moduleDefinition = job.raw_flow?.modules.find((m) => m.id === selectedJobStep)
		if (moduleDefinition?.value.type === 'forloopflow') {
			selectedJobStepType = 'forloop'
		} else if (moduleDefinition?.value.type === 'branchall') {
			selectedJobStepType = 'branchall'
			const newNames: [number, string][] = []
			moduleDefinition.value.branches.forEach((branch, idx) => {
				newNames.push([idx, emptyString(branch.summary) ? `Branch #${idx}` : branch.summary!])
			})
			restartBranchNames = newNames
		} else {
			selectedJobStepType = 'single'
		}

		// Read from the local — reading the `$state` we just wrote would register
		// it as a dependency of this effect, causing `effect_update_depth_exceeded`
		// because each write would reschedule the effect.
		if (isTopLevel) return

		// Inline-expanded subflow: id is `subflow:outerStep:[innerSubflow:...]<leaf>`.
		// The graph adds the `subflow:` prefix only for subflow expansions, so each
		// segment is a `Flow{path}` step. We walk the chain and, at the deepest
		// subflow, recurse into its cached modules to detect BranchOne / ForLoop
		// ancestors of the leaf so the chain captures locked branches and
		// iteration indices for those too.
		const subflowParse = parseExpandedSubflowId(selectedJobStep)
		if (subflowParse && job.raw_flow?.modules) {
			const top = job.raw_flow.modules.find((m) => m.id === subflowParse.subflowSteps[0])
			if (top && top.value.type === 'flow') {
				const innerPath: NestedRestartStep[] = []
				// Intermediate subflow boundaries: each is itself a Flow{path}, no
				// branch_or_iteration_n applies.
				for (const seg of subflowParse.subflowSteps.slice(1)) {
					innerPath.push({ step_id: seg })
				}

				// Look up the deepest subflow's modules in expandedSubflows. The graph
				// stores them under the prefixed key (the graph node id used at that
				// nesting level).
				const deepestKey =
					subflowParse.subflowSteps.length === 1
						? subflowParse.subflowSteps[0]
						: 'subflow:' + subflowParse.subflowSteps.join(':')
				const deepestModules = expandedSubflows[deepestKey]?.modules

				if (deepestModules) {
					const path = findStepPath(deepestModules, subflowParse.leaf)
					if (path) {
						// Same gating as the parent-flow case, minus the BranchOne
						// branch-mismatch check (subflow's flow_status isn't directly
						// reachable here; backend will still reject if the branch
						// wasn't taken). Parallel and unsupported containers are checked.
						const blocked = path.ancestors.some(
							(a) => a.type === 'branchall' || a.type === 'whileloopflow' || a.parallel === true
						)
						if (!blocked) {
							// Inside the subflow, the graph state for ForLoop ancestors
							// is keyed by the prefixed graph id. Build that key here so
							// `selectedForloopIndex` and `flow_jobs.length` lookups land
							// on the right entry (avoiding collisions with same-step-id
							// loops in the parent flow — e.g. parent has `e` with 4 iters
							// and the subflow also has `e` with 1 iter).
							const subflowPrefix = 'subflow:' + subflowParse.subflowSteps.join(':') + ':'
							const iterationFor = (stepId: string): number =>
								graphModuleStates[subflowPrefix + stepId]?.selectedForloopIndex ?? 0
							const iterCountFor = (stepId: string): number =>
								graphModuleStates[subflowPrefix + stepId]?.flow_jobs?.length ?? 0
							const counts: Record<string, number> = {}
							// Top-level (parent) container is the subflow step `h` —
							// not a ForLoop, no count.
							for (let i = 0; i < path.ancestors.length; i++) {
								const a = path.ancestors[i]
								const entry: NestedRestartStep = { step_id: a.stepId }
								if (a.type === 'forloopflow') {
									entry.branch_or_iteration_n = iterationFor(a.stepId)
									// Path key: each subflow boundary already contributes an
									// inner-N entry above (subflow steps without iterations).
									// Match the index that FlowRestartButton will use when
									// rendering iteration fields.
									counts[`inner-${innerPath.length}`] = iterCountFor(a.stepId)
								}
								innerPath.push(entry)
							}
							const leafEntry: NestedRestartStep = { step_id: subflowParse.leaf }
							if (path.target.value.type === 'forloopflow') {
								leafEntry.branch_or_iteration_n = iterationFor(subflowParse.leaf)
								counts[`inner-${innerPath.length}`] = iterCountFor(subflowParse.leaf)
							}
							innerPath.push(leafEntry)
							nestedRestartTopStepId = top.id
							nestedRestartPath = innerPath
							nestedPathIterationCounts = counts
							nestedRestartSupported = true
							return
						}
					}
				}

				// Fallback: subflow's modules not yet loaded (user clicked a step
				// without expanding the subflow first), or leaf not found / blocked.
				// Send a flat path; the backend will reject if the leaf is nested
				// inside an unsupported container.
				innerPath.push({ step_id: subflowParse.leaf })
				nestedRestartTopStepId = top.id
				nestedRestartPath = innerPath
				nestedRestartSupported = true
				return
			}
		}

		// BranchOne / sequential ForLoop within the parent's own flow_value. Walk
		// the tree to find the path; supported when ancestors are BranchOne /
		// sequential ForLoop only — and only when each BranchOne ancestor's chosen
		// branch in the original run actually contains the leaf, otherwise the
		// step never executed and the backend would error.
		if (!job.raw_flow?.modules) return
		const path = findStepPath(job.raw_flow.modules, selectedJobStep)
		if (!path || path.ancestors.length === 0) return
		const blocked = path.ancestors.some(
			(a) =>
				a.type === 'branchall' ||
				a.type === 'whileloopflow' ||
				a.parallel === true ||
				(a.type === 'branchone' && !branchOneAncestorMatchesOriginal(job, a))
		)
		if (blocked) return

		// For each ForLoop ancestor, default to the user's currently-open iteration
		// (`selectedForloopIndex`); fall back to 0. The popup surfaces every value
		// for confirmation/editing before submit, so we never silently send a guess.
		const iterationFor = (stepId: string): number =>
			graphModuleStates[stepId]?.selectedForloopIndex ?? 0
		const iterCountFor = (stepId: string): number =>
			graphModuleStates[stepId]?.flow_jobs?.length ?? 0
		const top = path.ancestors[0]
		const inner = path.ancestors.slice(1)
		const innerPath: NestedRestartStep[] = []
		const counts: Record<string, number> = {}
		for (const a of inner) {
			const entry: NestedRestartStep = { step_id: a.stepId }
			if (a.type === 'forloopflow') {
				entry.branch_or_iteration_n = iterationFor(a.stepId)
				counts[`inner-${innerPath.length}`] = iterCountFor(a.stepId)
			}
			innerPath.push(entry)
		}
		// If the SELECTED step is itself a ForLoop, include its iteration too so
		// the popup exposes a selector for it.
		const leafEntry: NestedRestartStep = { step_id: selectedJobStep }
		if (path.target.value.type === 'forloopflow') {
			leafEntry.branch_or_iteration_n = iterationFor(selectedJobStep)
			counts[`inner-${innerPath.length}`] = iterCountFor(selectedJobStep)
		}
		innerPath.push(leafEntry)

		nestedRestartTopStepId = top.stepId
		if (top.type === 'forloopflow') {
			nestedRestartTopBranchOrIterationN = iterationFor(top.stepId)
			counts['top'] = iterCountFor(top.stepId)
		}
		nestedRestartPath = innerPath
		nestedPathIterationCounts = counts
		nestedRestartSupported = true
	})

	const topLevelLoopIteration = $derived.by((): number | undefined => {
		const sel = opts.selectedJobStep()
		if (!sel) return undefined
		if ((selectedJobStepType as string) !== 'forloop') return undefined
		return opts.graphModuleStates()[sel]?.selectedForloopIndex
	})

	// `selectedJobStepIsTopLevel` answers "is this a top-level module structurally";
	// `topLevelRestartable` answers "AND should the restart button show". The
	// difference: parallel ForLoop / parallel BranchAll at the top level are
	// rejected by the backend, so we hide the button preemptively.
	const topLevelRestartable = $derived.by((): boolean => {
		if (!selectedJobStepIsTopLevel) return false
		const sel = opts.selectedJobStep()
		const job = opts.job()
		if (!sel || !job?.raw_flow?.modules) return false
		const mod = job.raw_flow.modules.find((m) => m.id === sel)
		if (!mod) return false
		const v = mod.value
		if (
			(v.type === 'forloopflow' || v.type === 'branchall' || v.type === 'whileloopflow') &&
			(v as { parallel?: boolean }).parallel === true
		) {
			return false
		}
		return true
	})

	// Iteration counts indexed by the graph module-state key (i.e. the prefixed
	// `subflow:...:<step_id>` for in-subflow loops, or the bare `step_id` for
	// top-level loops). Used by the popup for the SELECTED step's iteration
	// picker — that step is always at the unprefixed top level (the run page's
	// graph-state key matches the bare step_id), so the lookup is unambiguous.
	// For nested-path iteration fields, see `nestedPathIterationCounts` instead,
	// which is keyed by the popup's field-key ('top' / 'inner-N') and pulled
	// from the path-aware graph key — avoiding collisions when the same step
	// id appears at both the parent flow and inside a subflow.
	const iterationCounts = $derived.by((): Record<string, number> => {
		const out: Record<string, number> = {}
		for (const [id, state] of Object.entries(opts.graphModuleStates())) {
			const n = state.flow_jobs?.length
			if (typeof n !== 'number' || n <= 0) continue
			out[id] = n
		}
		return out
	})

	return {
		get selectedJobStepIsTopLevel() {
			return selectedJobStepIsTopLevel
		},
		get selectedJobStepType() {
			return selectedJobStepType
		},
		get restartBranchNames() {
			return restartBranchNames
		},
		get nestedRestartTopStepId() {
			return nestedRestartTopStepId
		},
		get nestedRestartTopBranchOrIterationN() {
			return nestedRestartTopBranchOrIterationN
		},
		get nestedRestartPath() {
			return nestedRestartPath
		},
		get nestedRestartSupported() {
			return nestedRestartSupported
		},
		get topLevelLoopIteration() {
			return topLevelLoopIteration
		},
		get topLevelRestartable() {
			return topLevelRestartable
		},
		get iterationCounts() {
			return iterationCounts
		},
		get nestedPathIterationCounts() {
			return nestedPathIterationCounts
		}
	}
}
