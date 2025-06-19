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
	stepsManuallySet = $state<Record<string, any>>({})

	constructor() {}

	getStepArgs(moduleId: string): Record<string, any> | undefined {
		return this.#steps[moduleId]
	}

	setStepArgs(moduleId: string, args: Record<string, any> | undefined) {
		this.#steps[moduleId] = args
	}

	setStepArg(moduleId: string, argName: string, value: any) {
		if (!this.#steps[moduleId]) {
			this.#steps[moduleId] = {}
		}
		this.#steps[moduleId][argName] = value
	}

	getStepArg(moduleId: string, argName: string): any | undefined {
		return this.#steps[moduleId]?.[argName]
	}

	setStepArgManually(moduleId: string, argName: string, value: any) {
		if (!this.stepsManuallySet[moduleId]) {
			this.stepsManuallySet[moduleId] = {}
		}
		this.stepsManuallySet[moduleId][argName] = value
	}

	setStepArgsManually(moduleId: string, args: Record<string, any>) {
		this.stepsManuallySet[moduleId] = args
	}

	getStepArgsManually(moduleId: string): Record<string, any> | undefined {
		return this.stepsManuallySet[moduleId]
	}

	getStepArgManually(moduleId: string, argName: string): any | undefined {
		return this.stepsManuallySet[moduleId]?.[argName]
	}

	resetArgManually(moduleId: string, argName: string) {
		if (this.stepsManuallySet[moduleId]) {
			delete this.stepsManuallySet[moduleId][argName]
		}
	}

	getMergedArgs(moduleId: string): Record<string, any> | undefined {
		return {
			...this.getStepArgs(moduleId),
			...this.getStepArgsManually(moduleId)
		}
	}

	updateArg(
		moduleId: string,
		argName: string,
		flowState: FlowState | undefined,
		flow: OpenFlow | undefined,
		previewArgs: Record<string, any> | undefined
	) {
		if (!flowState || !flow) {
			return
		}
		const modules = dfs(moduleId, flow, true)
		const previousModule = getPreviousModule(moduleId, flow)
		if (modules.length < 1) {
			return
		}
		let parentModule: FlowModule | undefined = undefined
		if (modules.length > 1) {
			parentModule = modules[modules.length - 1]
		}
		const stepPropPicker = getStepPropPicker(
			flowState,
			parentModule,
			previousModule,
			moduleId,
			flow,
			previewArgs,
			false
		)
		const pickableProperties = stepPropPicker.pickableProperties

		if (!this.#steps[moduleId]) {
			this.#steps[moduleId] = {}
		}
		this.#steps[moduleId][argName] = evalValue(
			argName,
			modules[0],
			this.#steps[moduleId] ?? {},
			pickableProperties,
			false
		)
		// Remove from manually set args if it was set manually
		if (this.stepsManuallySet[moduleId]) {
			delete this.stepsManuallySet[moduleId][argName]
		}
	}

	initializeFromSchema(
		mod: FlowModule,
		schema: { properties?: Record<string, any> },
		pickableProperties: PickableProperties | undefined
	) {
		const args = Object.fromEntries(
			Object.keys(schema.properties ?? {}).map((k) => [
				k,
				this.stepsManuallySet[mod.id]?.[k]
					? this.#steps[mod.id]?.[k]
					: evalValue(k, mod, this.#steps[mod.id] ?? {}, pickableProperties, false)
			])
		)
		this.setStepArgs(mod.id, structuredClone($state.snapshot(args)))
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
		if (modules.length > 1) {
			parentModule = modules[modules.length - 1]
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

	removeExtraKey(moduleId: string, keys: string[]) {
		const nargs = {}
		Object.keys(this.#steps[moduleId] ?? {}).forEach((key) => {
			if (keys.includes(key)) {
				nargs[key] = this.#steps[moduleId][key]
			}
		})
		this.#steps[moduleId] = nargs
	}
}
