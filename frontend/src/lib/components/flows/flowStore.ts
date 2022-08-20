import type { Flow } from '$lib/gen'
import { get, writable } from 'svelte/store'
import { flowStateStore, initFlowState } from './flowState'

export type FlowMode = 'push' | 'pull'

export const flowStore = writable<Flow>(undefined)

export function initFlow(flow: Flow) {
	flowStore.set(flow)
	initFlowState(flow)
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
