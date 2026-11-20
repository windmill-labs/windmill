import { getContext, hasContext, setContext } from 'svelte'
import { SvelteMap } from 'svelte/reactivity'
import type { Job } from '$lib/gen'
import type { GraphModuleState } from './model'

const FLOW_RUN_STATUS_KEY = 'FlowRunStatus'

export type SuspendStatus = Record<string, { job: Job; nb: number }>

/**
 * Run status is read straight from here by the node and edge renderers rather than
 * being carried in their `data`. Baking it into `data` means the only way to show a
 * status change is to rebuild every node and edge, which re-runs the sugiyama layout
 * and makes xyflow re-measure and re-create the whole graph on every poll.
 */
export class FlowRunStatus {
	#moduleStates = new SvelteMap<string, GraphModuleState>()
	flowJob = $state.raw<Job | undefined>(undefined)
	suspendStatus = $state.raw<SuspendStatus>({})
	/** Whether the graph draws a run, as opposed to a flow definition with no run states. */
	showsRun = $state(false)
	/** The run has finished, so a step with no state never ran rather than not yet. */
	runDone = $state(false)

	getModuleState(id: string | undefined): GraphModuleState | undefined {
		return id == undefined ? undefined : this.#moduleStates.get(id)
	}

	/** A step the finished run never reached. Undecided while the run goes on, since a step not
	 * reached yet may still run. */
	didNotRun(id: string): boolean {
		if (!this.runDone) return false
		const state = this.getModuleState(id)
		return !state || state.type === 'WaitingForPriorSteps' || !!state.skipped
	}

	/** Steps inside the subflow step `id`, at any depth, that were retried in the shown attempt. */
	retriedStepsInside(id: string): number {
		const prefix = (id.startsWith('subflow:') ? id : 'subflow:' + id) + ':'
		let count = 0
		for (const [key, state] of this.#moduleStates) {
			if (key.startsWith(prefix) && (state.retries ?? 0) > 0) count++
		}
		return count
	}

	setModuleStates(next: Record<string, GraphModuleState> | undefined) {
		const incoming = next ?? {}
		for (const id of [...this.#moduleStates.keys()]) {
			if (!(id in incoming)) {
				this.#moduleStates.delete(id)
			}
		}
		// Writing a key invalidates only that key's readers, so one step finishing
		// never re-renders the other steps.
		for (const [id, state] of Object.entries(incoming)) {
			if (this.#moduleStates.get(id) !== state) {
				this.#moduleStates.set(id, state)
			}
		}
	}
}

export function setFlowRunStatusContext(): FlowRunStatus {
	const status = new FlowRunStatus()
	setContext(FLOW_RUN_STATUS_KEY, status)
	return status
}

/**
 * Graphs that never show run status (mini graph, diff viewer) provide no context, so
 * every reader has to tolerate its absence.
 */
export function getFlowRunStatusContext(): FlowRunStatus | undefined {
	return hasContext(FLOW_RUN_STATUS_KEY)
		? getContext<FlowRunStatus>(FLOW_RUN_STATUS_KEY)
		: undefined
}
