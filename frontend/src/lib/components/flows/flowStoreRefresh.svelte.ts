import type { OpenFlow } from '$lib/gen'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import { reanchorAgentEditsAcross } from './agentEditStore.svelte'

// Refresh the flow store without dropping an in-progress agent Editing session: use at every
// content-preserving refresh site (structural edits, schema/failure/mock changes) where a bare
// refreshStateStore clone would break the edit marker's array identity (see agentEditStore).
// Content-changing replacements (undo, YAML/diff/AI apply, session restores) stay bare so stale
// edit state keeps invalidating.
export function refreshFlowStateStore(flowStore: StateStore<OpenFlow>) {
	reanchorAgentEditsAcross(
		() => flowStore.val.value?.modules,
		() => refreshStateStore(flowStore)
	)
}
