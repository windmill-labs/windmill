import type { FlowModule, OpenFlow } from '$lib/gen'
import type { FlowState } from './flowState'
import {
	dfs,
	getPreviousModule,
	getStepPropPicker,
	type PickableProperties
} from './previousResults'
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

	updateStepArgs(
		id: string,
		flowState: FlowState | undefined,
		flow: OpenFlow | undefined,
		previewArgs: Record<string, any> | undefined
	) {
		if (!flowState || !flow) {
			return
		}
		const modules = dfs(id, flow, true)
		const previousModule = getPreviousModule(id, flow)
		if (modules.length < 1) {
			return
		}
		let parentModule: FlowModule | undefined = undefined
		if (modules.length > 2) {
			parentModule = modules[-1]
		}
		const stepPropPicker = getStepPropPicker(
			flowState,
			parentModule,
			previousModule,
			id,
			flow,
			previewArgs,
			false
		)
		const pickableProperties = stepPropPicker.pickableProperties
		this.initializeFromSchema(modules[0], flowState[id]?.schema ?? {}, pickableProperties)
	}
}
