import type { Job } from '$lib/gen'
import type { GraphModuleState } from './graph'

export type ModuleTestState = {
	loading: boolean
	cancel?: () => Promise<void>
	testJob?: Job
	hiddenInGraph?: boolean
}

export class ModulesTestStates {
	states: Record<string, ModuleTestState> = $state({})
	runTestCb?: (moduleId: string) => void
	hideJobsInGraph() {
		for (const state of Object.values(this.states)) {
			state.hiddenInGraph = true
		}
	}
	constructor(runTestCb?: (moduleId: string) => void) {
		this.states = {}
		this.runTestCb = runTestCb
	}
}

export function jobToGraphModuleState(testState: ModuleTestState): GraphModuleState | undefined {
	if (testState.hiddenInGraph) {
		return undefined
	} else if (testState.loading) {
		return {
			type: 'InProgress',
			args: {}
		}
	} else if (testState.testJob) {
		return {
			args: testState.testJob.args,
			type:
				testState.testJob.type === 'QueuedJob'
					? 'InProgress'
					: testState.testJob['success']
						? 'Success'
						: 'Failure',
			job_id: testState.testJob.id,
			tag: testState.testJob.tag,
			duration_ms: testState.testJob['duration_ms'],
			started_at: testState.testJob.started_at
				? new Date(testState.testJob.started_at).getTime()
				: undefined
		}
	}
}
