import type { Job } from '$lib/gen'
import type { GraphModuleState } from './graph'
import { findStepPath, parseExpandedSubflowId } from './restartFromStepPath'
import { emptyString } from '$lib/utils'

export type NestedRestartStep = { step_id: string; branch_or_iteration_n?: number }

/**
 * Reactive state shared by the run page and the editor's flow preview to drive
 * the "restart flow at step" UI. Given the currently selected step, the
 * completed flow job, and the graph's per-module display state, derives:
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
 *   - `iterationCounts` — map of ForLoop step id → number of recorded
 *     iterations, so the popup can render an iteration `<select>` matching
 *     the graph's tabs instead of a free-form number input
 *
 * Inputs are passed as getters so the composable stays reactive in either
 * caller's signal graph.
 */
export function useNestedRestartState(opts: {
	selectedJobStep: () => string | undefined
	job: () => Job | undefined
	graphModuleStates: () => Record<string, GraphModuleState>
}) {
	let selectedJobStepIsTopLevel: boolean | undefined = $state(undefined)
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = $state('single')
	let restartBranchNames: [number, string][] = $state([])
	let nestedRestartTopStepId: string | undefined = $state(undefined)
	let nestedRestartTopBranchOrIterationN: number | undefined = $state(undefined)
	let nestedRestartPath: NestedRestartStep[] | undefined = $state(undefined)
	let nestedRestartSupported = $state(false)

	$effect(() => {
		const selectedJobStep = opts.selectedJobStep()
		const job = opts.job()
		const graphModuleStates = opts.graphModuleStates()

		nestedRestartTopStepId = undefined
		nestedRestartTopBranchOrIterationN = undefined
		nestedRestartPath = undefined
		nestedRestartSupported = false
		restartBranchNames = []

		if (selectedJobStep === undefined || job?.flow_status?.modules === undefined) {
			return
		}

		selectedJobStepIsTopLevel =
			job.flow_status.modules.findIndex((m) => m.id === selectedJobStep) >= 0
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

		if (selectedJobStepIsTopLevel) return

		// Inline-expanded subflow: id is `subflow:outerStep:[innerSubflow:...]<leaf>`.
		// The graph adds the `subflow:` prefix only for subflow expansions, so each
		// segment is a `Flow{path}` step. The backend will validate that the leaf
		// exists at that path.
		const subflowParse = parseExpandedSubflowId(selectedJobStep)
		if (subflowParse && job.raw_flow?.modules) {
			const top = job.raw_flow.modules.find((m) => m.id === subflowParse.subflowSteps[0])
			if (top && top.value.type === 'flow') {
				const innerPath: NestedRestartStep[] = []
				for (const seg of subflowParse.subflowSteps.slice(1)) {
					innerPath.push({ step_id: seg })
				}
				innerPath.push({ step_id: subflowParse.leaf })
				nestedRestartTopStepId = top.id
				nestedRestartPath = innerPath
				nestedRestartSupported = true
				return
			}
		}

		// BranchOne / sequential ForLoop within the parent's own flow_value. Walk
		// the tree to find the path; supported when ancestors are BranchOne /
		// sequential ForLoop only.
		if (!job.raw_flow?.modules) return
		const path = findStepPath(job.raw_flow.modules, selectedJobStep)
		if (!path || path.ancestors.length === 0) return
		const blocked = path.ancestors.some((a) => a.type === 'branchall' || a.type === 'whileloopflow')
		if (blocked) return

		// For each ForLoop ancestor, default to the user's currently-open iteration
		// (`selectedForloopIndex`); fall back to 0. The popup surfaces every value
		// for confirmation/editing before submit, so we never silently send a guess.
		const iterationFor = (stepId: string): number =>
			graphModuleStates[stepId]?.selectedForloopIndex ?? 0
		const top = path.ancestors[0]
		const inner = path.ancestors.slice(1)
		const innerPath: NestedRestartStep[] = []
		for (const a of inner) {
			const entry: NestedRestartStep = { step_id: a.stepId }
			if (a.type === 'forloopflow') {
				entry.branch_or_iteration_n = iterationFor(a.stepId)
			}
			innerPath.push(entry)
		}
		// If the SELECTED step is itself a ForLoop, include its iteration too so
		// the popup exposes a selector for it.
		const leafEntry: NestedRestartStep = { step_id: selectedJobStep }
		if (path.target.value.type === 'forloopflow') {
			leafEntry.branch_or_iteration_n = iterationFor(selectedJobStep)
		}
		innerPath.push(leafEntry)

		nestedRestartTopStepId = top.stepId
		nestedRestartTopBranchOrIterationN =
			top.type === 'forloopflow' ? iterationFor(top.stepId) : undefined
		nestedRestartPath = innerPath
		nestedRestartSupported = true
	})

	const topLevelLoopIteration = $derived.by((): number | undefined => {
		const sel = opts.selectedJobStep()
		if (!sel) return undefined
		if ((selectedJobStepType as string) !== 'forloop') return undefined
		return opts.graphModuleStates()[sel]?.selectedForloopIndex
	})

	const iterationCounts = $derived.by((): Record<string, number> => {
		const out: Record<string, number> = {}
		for (const [id, state] of Object.entries(opts.graphModuleStates())) {
			const n = state.flow_jobs?.length
			if (typeof n === 'number' && n > 0) {
				out[id] = n
			}
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
		get iterationCounts() {
			return iterationCounts
		}
	}
}
