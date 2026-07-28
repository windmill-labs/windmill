import type { OpenFlow } from '$lib/gen'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import { reanchorAgentEditsAcross } from './agentEditStore.svelte'

// Use at content-preserving refresh sites (structural edits, schema/failure/mock changes): a bare
// refreshStateStore clone would break the agent edit marker's array identity. Content-changing
// replacements (undo, YAML/diff/AI apply, session restores) must stay bare so stale edit state
// keeps invalidating.
export function refreshFlowStateStore(flowStore: StateStore<OpenFlow>) {
	reanchorAgentEditsAcross(
		() => flowStore.val.value?.modules,
		() => refreshStateStore(flowStore)
	)
}
