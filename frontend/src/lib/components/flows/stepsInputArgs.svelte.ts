import type { FlowModule, OpenFlow } from '$lib/gen'
import type { FlowState } from './flowState'
import {
	dfs,
	getPreviousModule,
	getStepPropPicker,
	type PickableProperties
} from './previousResults'
import { evalValue } from './utils.svelte'

export class StepsInputArgs {
	#stepsEvaluated = $state<Record<string, Record<string, any>>>({})
	#steps = $state<Record<string, Record<string, any>>>({})

	constructor() {}

	setStepArgsManually(moduleId: string, args: Record<string, any>) {
		this.#steps[moduleId] = args
	}

	getStepArgs(moduleId: string): Record<string, any> | undefined {
		return this.#steps[moduleId]
	}

	getStepInputArgs(moduleId: string, argName: string): any | undefined {
		return this.#steps[moduleId]?.[argName]
	}

	setStepArgs(moduleId: string, args: Record<string, any>) {
		this.#steps[moduleId] = args
	}

	setStepInputArgs(moduleId: string, argName: string, value: any) {
		if (!this.#steps[moduleId]) {
			this.#steps[moduleId] = {}
		}
		this.#steps[moduleId][argName] = value
	}

	getStepArg(moduleId: string, argName: string): any | undefined {
		return this.#steps[moduleId]?.[argName]
	}

	setEvaluatedStepArg(moduleId: string, argName: string, value: any) {
		if (!this.#steps[moduleId]) {
			this.#steps[moduleId] = {}
		}
		if (!this.#stepsEvaluated[moduleId]) {
			this.#stepsEvaluated[moduleId] = {}
		}
		this.#steps[moduleId][argName] = $state.snapshot(value)
		this.#stepsEvaluated[moduleId][argName] = $state.snapshot(value)
	}

	isArgManuallySet(moduleId: string, argName: string): boolean {
		try {
			const stepsValue = this.#steps[moduleId]?.[argName]
			const evaluatedValue = this.#stepsEvaluated[moduleId]?.[argName]

			const stepsStr = JSON.stringify(stepsValue)
			const evaluatedStr = JSON.stringify(evaluatedValue)
			return stepsStr !== evaluatedStr
		} catch (error) {
			console.warn(
				`Error in isArgManuallySet for moduleId: ${moduleId}, argName: ${argName}`,
				error
			)
			return this.#steps[moduleId]?.[argName] !== this.#stepsEvaluated[moduleId]?.[argName]
		}
	}

	getManuallyEditedArgs(moduleId: string): string[] {
		const manuallyEditedArgs: string[] = []

		const moduleArgs = this.#steps[moduleId] ?? {}

		Object.keys(moduleArgs).forEach((argName) => {
			if (this.isArgManuallySet(moduleId, argName)) {
				manuallyEditedArgs.push(argName)
			}
		})
		return manuallyEditedArgs
	}

	/*
		Evaluate the arg value from the flow state and replace the test value.
	*/
	evalArg(
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

		const argSnapshot = $state.snapshot(evalValue(argName, modules[0], pickableProperties, false))
		this.#stepsEvaluated[moduleId][argName] = argSnapshot
		this.#steps[moduleId][argName] = structuredClone(argSnapshot)
	}

	initializeFromSchema(
		mod: FlowModule,
		schema: { properties?: Record<string, any> },
		pickableProperties: PickableProperties | undefined
	) {
		const args = Object.fromEntries(
			Object.keys(schema.properties ?? {}).map((k) => [
				k,
				evalValue(k, mod, pickableProperties, false)
			])
		)

		const manuallyEditedArgs = this.getManuallyEditedArgs(mod.id)

		if (!this.#steps[mod.id]) {
			this.#steps[mod.id] = {}
		}
		if (!this.#stepsEvaluated[mod.id]) {
			this.#stepsEvaluated[mod.id] = {}
		}
		this.#stepsEvaluated[mod.id] = $state.snapshot(args)

		// Preserve manually edited args
		const argsSnapshot = $state.snapshot(args)
		Object.keys(argsSnapshot).forEach((key) => {
			if (manuallyEditedArgs.includes(key)) {
				argsSnapshot[key] = this.#steps[mod.id]?.[key]
			}
		})
		this.#steps[mod.id] = argsSnapshot
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
		if (!this.#stepsEvaluated[moduleId]) {
			return
		}
		const nargs = {}
		Object.keys(this.#stepsEvaluated[moduleId] ?? {}).forEach((key) => {
			if (keys.includes(key)) {
				nargs[key] = this.#stepsEvaluated[moduleId]?.[key]
			}
		})
		this.#stepsEvaluated[moduleId] = nargs
	}

	resetManuallyEditedArgs() {
		this.#steps = $state.snapshot(this.#stepsEvaluated)
	}
}
