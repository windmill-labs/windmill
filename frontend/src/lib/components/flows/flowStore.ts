import { ScriptService, type Flow } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { derived, get, writable } from 'svelte/store'
import { flowStateStore, type FlowState } from './flowState'

export type FlowMode = 'push' | 'pull'

export const mode = writable<FlowMode>('push')
export const flowStore = writable<Flow>(undefined)

export function initFlow(flow: Flow) {
	flowStore.set(flow)
}

export const isCopyFirstStepSchemaDisabled = derived(flowStore, (flow: Flow | undefined) => {
	if (flow) {
		const modules = flow.value.modules
		const [firstModule] = modules

		return (
			modules.length === 0 || (firstModule.value.type === 'script' && firstModule.value.path === '')
		)
	} else {
		return true
	}
})

export async function copyFirstStepSchema() {
	const flowState = get(flowStateStore)
	flowStore.update((flow) => {
		if (flowState[0].schema) {
			flow.schema = flowState[0].schema
		}
		return flow
	})
}

export async function findNextAvailablePath(path: string): Promise<string> {
	try {
		await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path
		})

		const [_, version] = path.split(/.*_([0-9]*)/)

		if (version.length > 0) {
			path = path.slice(0, -(version.length + 1))
		}

		path = `${path}_${Number(version) + 1}`

		return findNextAvailablePath(path)
	} catch (e) {
		// Catching an error means the path is available
		return path
	}
}
