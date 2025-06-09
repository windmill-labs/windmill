import type { Flow, OpenFlow } from '$lib/gen'
import { writable, type Writable } from 'svelte/store'
import { type FlowState } from './flowState'
import { sendUserToast } from '$lib/toast'

export type FlowMode = 'push' | 'pull'

export const importFlowStore = writable<Flow | undefined>(undefined)

export async function copyFirstStepSchema(flowState: FlowState, flowStore: Writable<OpenFlow>) {
	flowStore.update((flow) => {
		const firstModuleId = flow.value.modules[0]?.id

		if (flowState[firstModuleId] && firstModuleId) {
			flow.schema = structuredClone(flowState[firstModuleId].schema)
			const v = flow.value.modules[0].value
			if (v.type == 'rawscript' || v.type == 'script') {
				Object.keys(v.input_transforms ?? {}).forEach((key) => {
					v.input_transforms[key] = {
						type: 'javascript',
						expr: `flow_input.${key}`
					}
				})
				return flow
			}
			sendUserToast('Only scripts can be used as a input schema', true)
			return flow
		}
		sendUserToast('No first step found', true)
		return flow
	})
}

export async function getFirstStepSchema(flowState: FlowState, flow: OpenFlow) {
	const firstModuleId = flow.value.modules[0]?.id

	if (!firstModuleId || !flowState[firstModuleId]) {
		throw new Error('no first step found')
	}

	const schema = structuredClone(flowState[firstModuleId].schema)
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
