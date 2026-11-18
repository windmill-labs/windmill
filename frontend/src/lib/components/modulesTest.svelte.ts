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
	}
	// Testing one step runs it as a single-module flow preview, so an agent's calls land on that
	// preview's own status. They arrive while it is still running, which is when they are worth
	// showing, so they have to be read before the loading short-circuit.
	const agentModule = testState.testJob?.['flow_status']?.modules?.[0]
	const agentActions = agentModule?.agent_actions
		? {
				agent_actions: agentModule.agent_actions,
				agent_actions_success: agentModule.agent_actions_success
			}
		: undefined
	if (testState.loading) {
		return {
			type: 'InProgress',
			args: {},
			...agentActions
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
				: undefined,
			...agentActions
		}
	}
}
