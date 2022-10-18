import type { Flow, FlowModule, ForloopFlow, InputTransform } from '$lib/gen'
import { get, writable } from 'svelte/store'
import { flowStateStore, initFlowState } from './flowState'

export type FlowMode = 'push' | 'pull'

export const flowStore = writable<Flow>({
	summary: '',
	value: { modules: [] },
	path: '',
	edited_at: '',
	edited_by: '',
	archived: false,
	extra_perms: {}
})

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
		let modVal = mod as FlowModule & {
			input_transform?: Record<string, InputTransform>
			stop_after_if_expr?: string
			skip_if_stopped?: boolean
		}
		if (modVal.input_transform) {
			modVal.input_transforms = modVal.input_transform
			delete modVal.input_transform
		}
		if (modVal.input_transforms && modVal.value.type == 'script' || modVal.value.type == 'rawscript') {
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

export async function copyFirstStepSchema() {
	const flowState = get(flowStateStore)
	flowStore.update((flow) => {
		if (flowState.modules[0].schema) {
			flow.schema = flowState.modules[0].schema
		}
		return flow
	})
}
