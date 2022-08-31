import type { Flow, FlowModule, ForloopFlow, InputTransform } from '$lib/gen'
import { get, writable } from 'svelte/store'
import { flowStateStore, initFlowState } from './flowState'

export type FlowMode = 'push' | 'pull'

export const flowStore = writable<Flow>(undefined)

export function initFlow(flow: Flow) {
	for (const mod of flow.value.modules) {
		migrateFlowModule(mod)
		let val = mod.value
		if (val.type == 'forloopflow') {
			let flowVal = val as ForloopFlow & { value?: { modules?: FlowModule[] } }
			if (flowVal.value && flowVal.value.modules) {
				flowVal.modules = flowVal.value.modules
				flowVal.modules.forEach(migrateFlowModule)
				flowVal.value = undefined
			}
		}
	}
	flowStore.set(flow)
	initFlowState(flow)

	function migrateFlowModule(mod: FlowModule) {
		let modVal = mod as FlowModule & { input_transform?: Record<string, InputTransform>} 
		if (modVal.input_transform) {
			modVal.input_transforms = modVal.input_transform
			modVal.input_transform = undefined
		}
		if (modVal.stop_after_if_expr) {
			modVal.stop_after_if = modVal.stop_after_if ?? {};
			modVal.stop_after_if.expr = modVal.stop_after_if_expr;
			delete modVal.stop_after_if_expr;
		}
		if (modVal.skip_if_stopped) {
			modVal.stop_after_if = modVal.stop_after_if ?? {};
			modVal.stop_after_if.skip_if_stopped = modVal.skip_if_stopped;
			delete modVal.skip_if_stopped;
		}
	}
}

export async function copyFirstStepSchema() {
	const flowState = get(flowStateStore)
	flowStore.update((flow) => {
		if (flowState[0].schema) {
			flow.schema = flowState[0].schema
		}
		return flow
	})
}
