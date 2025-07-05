type moduleTestState = {
	loading: boolean
	cancel?: () => Promise<void>
}

export class ModulesTestStates {
	states: Record<string, moduleTestState> = $state({})

	constructor() {
		this.states = {}
	}
}
