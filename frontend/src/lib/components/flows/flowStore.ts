import { ScriptService, type Flow } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { derived, get, writable } from 'svelte/store'
import { getFirstStepSchema } from './utils'

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
	const flow = get(flowStore)

	const flowSchema = await getFirstStepSchema(flow)

	flowStore.update((flow: Flow) => {
		flow.schema = flowSchema
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
