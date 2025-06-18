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
	#manuallySetArgs = $state<Record<string, Record<string, boolean>>>({})

	constructor() {}

	getStepArgs(moduleId: string): Record<string, any> | undefined {
		return this.#steps[moduleId]
	}

	setStepArgs(moduleId: string, args: Record<string, any> | undefined) {
		this.#steps[moduleId] = args
	}

	setArgManually(moduleId: string, argName: string, value: any) {
		if (!this.#manuallySetArgs[moduleId]) {
			this.#manuallySetArgs[moduleId] = {}
		}
		this.#manuallySetArgs[moduleId][argName] = true
		if (!this.#steps[moduleId]) {
			this.#steps[moduleId] = {}
		}
		this.#steps[moduleId][argName] = value
	}

	get manuallySetArgs() {
		return this.#manuallySetArgs
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
		console.log('dbg updateArg', moduleId, argName, this.#manuallySetArgs)
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
		if (this.#manuallySetArgs[moduleId]) {
			delete this.#manuallySetArgs[moduleId][argName]
		}
		console.log('dbg updateArg done', this.#manuallySetArgs)
	}

	initializeFromSchema(
		mod: FlowModule,
		schema: { properties?: Record<string, any> },
		pickableProperties: PickableProperties | undefined
	) {
		const args = Object.fromEntries(
			Object.keys(schema.properties ?? {}).map((k) => [
				k,
				this.#manuallySetArgs[mod.id]?.[k]
					? this.#steps[mod.id]?.[k]
					: evalValue(k, mod, this.#steps[mod.id] ?? {}, pickableProperties, false)
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
}
