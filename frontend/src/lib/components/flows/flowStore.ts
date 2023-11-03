import type { Flow, OpenFlow } from '$lib/gen'
import { writable, type Writable } from 'svelte/store'
import { initFlowState, type FlowState } from './flowState'

export type FlowMode = 'push' | 'pull'

export const importFlowStore = writable<Flow | undefined>(undefined)

export async function initFlow(
	flow: Flow,
	flowStore: Writable<Flow>,
	flowStateStore: Writable<FlowState>
) {
	await initFlowState(flow, flowStateStore)
	flowStore.set(flow)
}

export async function copyFirstStepSchema(flowState: FlowState, flowStore: Writable<OpenFlow>) {
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
