import type { Job } from '$lib/gen'

type moduleTestState = {
	loading: boolean
	cancel?: () => Promise<void>
	testJob?: Job
}

export class ModulesTestStates {
	states: Record<string, moduleTestState> = $state({})

	constructor() {
		this.states = {}
	}
}
