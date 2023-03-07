import type { Flow, FlowModule, ForloopFlow, InputTransform } from '$lib/gen'
import { get, writable, type Writable } from 'svelte/store'
import { initFlowState, type FlowState } from './flowState'
import { numberToChars } from './utils'

export type FlowMode = 'push' | 'pull'

export const importFlowStore = writable<Flow | undefined>(undefined)


export function dfs<T>(modules: FlowModule[], f: (x: FlowModule) => T): T[] {
	let result: T[] = []
	for (const module of modules) {
		if (module.value.type == 'forloopflow') {
			result = result.concat(f(module))
			result = result.concat(dfs(module.value.modules, f))
		} else if (module.value.type == 'branchone') {
			result = result.concat(f(module))
			result = result.concat(dfs(module.value.branches.map((b) => b.modules).flat().concat(module.value.default), f))
		} else if (module.value.type == 'branchall') {
			result = result.concat(f(module))
			result = result.concat(dfs(module.value.branches.map((b) => b.modules).flat(), f))
		} else {
			result.push(f(module))
		}
	}
	return result
}


export async function initFlow(flow: Flow, flowStore: Writable<Flow>, flowStateStore: Writable<FlowState>) {
	let counter = 0
	for (const mod of flow.value.modules) {
		migrateFlowModule(mod)
		let val = mod.value
		if (val.type == 'forloopflow') {
			let flowVal = val as ForloopFlow & { value?: { modules?: FlowModule[] } }
			if (flowVal.value && flowVal.value.modules) {
				flowVal.modules = flowVal.value.modules
				flowVal.value = undefined
			}
			flowVal.modules.forEach(migrateFlowModule)

		}
	}

	await initFlowState(flow, flowStateStore)
	flowStore.set(flow)

	function migrateFlowModule(mod: FlowModule) {
		if (mod.id == undefined) {
			mod.id = numberToChars(counter++)
		}
		let modVal = mod as FlowModule & {
			input_transform?: Record<string, InputTransform>
			stop_after_if_expr?: string
			skip_if_stopped?: boolean
		}
		if (modVal.input_transform) {
			modVal.input_transforms = modVal.input_transform
			delete modVal.input_transform
		}
		if (
			(modVal.input_transforms && modVal.value.type == 'script') ||
			modVal.value.type == 'rawscript'
		) {
			if (modVal.input_transforms && Object.keys(modVal.input_transforms).length > 0) {
				modVal.value.input_transforms = modVal.input_transforms
				delete modVal.input_transforms
			}
		}
		if (modVal.stop_after_if_expr) {
			modVal.stop_after_if = {
				expr: modVal.stop_after_if_expr,
				skip_if_stopped: modVal.skip_if_stopped
			}
			delete modVal.stop_after_if_expr
			delete modVal.skip_if_stopped
		}
	}
}

export async function copyFirstStepSchema(flowState: FlowState, flowStore: Writable<Flow>) {
	flowStore.update((flow) => {
		const firstModuleId = flow.value.modules[0]?.id

		if (flowState[firstModuleId] && firstModuleId) {
			flow.schema = flowState[firstModuleId].schema
			const v = flow.value.modules[0].value
			if (v.type == 'rawscript' || v.type == 'script') {
				Object.keys(v.input_transforms ?? {}).forEach((key) => {
					v.input_transforms[key] = {
						type: 'javascript',
						expr: `flow_input.${key}`
					}
				})
			}
		}
		return flow
	})
}
