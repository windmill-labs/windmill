import type { Job } from '$lib/gen'

type moduleTestState = {
	loading: boolean
	cancel?: () => Promise<void>
	testJob?: Job
}

export class ModulesTestStates {
	states: Record<string, moduleTestState> = $state({})
	runTestCb?: (moduleId: string) => void

	constructor(runTestCb?: (moduleId: string) => void) {
		this.states = {}
		this.runTestCb = runTestCb
	}
}
