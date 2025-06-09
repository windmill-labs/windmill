import type { FlowModule } from '$lib/gen'
import type { PickableProperties } from './previousResults'
import { evalValue } from './utils'

export class TestSteps {
	#steps = $state<Record<string, any>>({})

	constructor() {}

	getStepArgs(moduleId: string): Record<string, any> | undefined {
		return this.#steps[moduleId]
	}

	setStepArgs(moduleId: string, args: Record<string, any> | undefined) {
		this.#steps[moduleId] = args
	}

	initializeFromSchema(
		mod: FlowModule,
		schema: { properties?: Record<string, any> },
		pickableProperties: PickableProperties | undefined
	) {
		const args = Object.fromEntries(
			Object.keys(schema.properties ?? {}).map((k) => [
				k,
				evalValue(k, mod, this.#steps, pickableProperties, false)
			])
		)
		this.setStepArgs(mod.id, args)
	}
}
