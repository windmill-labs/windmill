import type { FlowModule, Job } from '$lib/gen'
import type { GraphModuleState } from './graph'
import { buildNestedRestartPath, type NestedRestartStep } from './restartFromStepPath'
import { emptyString } from '$lib/utils'

export type { NestedRestartStep }

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
		if (!job.raw_flow?.modules) return

		// Flatten container nesting (ForLoop/BranchOne) and subflow boundaries into
		// one chain from a top-level container down to the leaf. See
		// `buildNestedRestartPath` for why walking each level is required.
		const result = buildNestedRestartPath({
			selectedJobStep,
			rawFlowModules: job.raw_flow.modules,
			flowStatusModules: job.flow_status.modules,
			graphModuleStates,
			expandedSubflows
		})
		if (!result) return

		nestedRestartTopStepId = result.topStepId
		nestedRestartTopBranchOrIterationN = result.topBranchOrIterationN
		nestedRestartPath = result.path
		nestedPathIterationCounts = result.iterationCounts
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
