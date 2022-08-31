import type { Flow, FlowModule, ForloopFlow, InputTransform } from '$lib/gen'
import { get, writable } from 'svelte/store'
import { flowStateStore, initFlowState } from './flowState'

export type FlowMode = 'push' | 'pull'

export const flowStore = writable<Flow>(undefined)

export function initFlow(flow: Flow) {
	for (const mod of flow.value.modules) {
		migrateInitTransform(mod)
		let val = mod.value
		if (val.type == 'forloopflow') {
			let flowVal = val as ForloopFlow & { value?: { modules?: FlowModule[] } }
			if (flowVal.value && flowVal.value.modules) {
				flowVal.modules = flowVal.value.modules
				flowVal.modules.forEach(migrateInitTransform)
				flowVal.value = undefined
			}
		}
	}
	flowStore.set(flow)
	initFlowState(flow)

	function migrateInitTransform(mod: FlowModule) {
		let modVal = mod as FlowModule & { input_transform?: Record<string, InputTransform>} 
		if (modVal.input_transform) {
			modVal.input_transforms = modVal.input_transform
			modVal.input_transform = undefined
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
