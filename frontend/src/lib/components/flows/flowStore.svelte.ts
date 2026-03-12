import type { Flow, OpenFlow } from '$lib/gen'
import { writable } from 'svelte/store'
import { initFlowState, type FlowState } from './flowState'
import { sendUserToast } from '$lib/toast'
import type { StateStore } from '$lib/utils'
export type FlowMode = 'push' | 'pull'

export const importFlowStore = writable<Flow | undefined>(undefined)

export async function initFlow(
	flow: Flow,
	flowStore: StateStore<Flow>,
	flowStateStore: StateStore<FlowState>
) {
	await initFlowState(flow, flowStateStore)
	flowStore.val = flow
}

export async function copyFirstStepSchema(
	flowState: FlowState,
	flowStore: StateStore<OpenFlow>
): Promise<void> {
	const firstModuleId = flowStore.val.value.modules[0]?.id

	if (flowState[firstModuleId] && firstModuleId) {
		flowStore.val.schema = structuredClone($state.snapshot(flowState[firstModuleId].schema))
		const v = flowStore.val.value.modules[0].value
		if (v.type == 'rawscript' || v.type == 'script') {
			Object.keys(v.input_transforms ?? {}).forEach((key) => {
				v.input_transforms[key] = {
					type: 'javascript',
					expr: `flow_input.${key}`
				}
			})
			return
		}
		sendUserToast('Only scripts can be used as a input schema', true)
		return
	}
	sendUserToast('No first step found', true)
	return
}

export async function getFirstStepSchema(flowState: FlowState, flow: OpenFlow) {
	const firstModuleId = flow.value.modules[0]?.id

	if (!firstModuleId || !flowState[firstModuleId]) {
		throw new Error('no first step found')
	}

	const schema = $state.snapshot(flowState[firstModuleId].schema)
	const v = flow.value.modules[0].value

	if (v.type !== 'rawscript' && v.type !== 'script') {
		throw new Error('only scripts can be used as a input schema')
	}

	const simplifiedModule = {
		id: flow.value.modules[0].id,
		summary: flow.value.modules[0].summary,
		value: {
			type: flow.value.modules[0].value.type,
			...('path' in flow.value.modules[0].value ? { path: flow.value.modules[0].value.path } : {}),
			...('language' in flow.value.modules[0].value
				? { language: flow.value.modules[0].value.language }
				: {})
		}
	}

	return {
		schema,
		mod: simplifiedModule,
		connectFirstNode: () => {
			Object.keys(v.input_transforms ?? {}).forEach((key) => {
				v.input_transforms[key] = {
					type: 'javascript',
					expr: `flow_input.${key}`
				}
			})
		}
	}
}

export function replaceId(expr: string, id: string, newId: string): string {
	return expr
		.replaceAll(`results.${id}`, `results.${newId}`)
		.replaceAll(`results?.${id}`, `results?.${newId}`)
}
