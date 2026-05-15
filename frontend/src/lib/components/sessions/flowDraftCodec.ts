import type { Flow } from '$lib/gen'
import type { FlowDraftValue } from '$lib/components/copilot/chat/global/draftStore.svelte'

// Convert the editor's full `Flow` (carrying metadata like path, edited_by,
// edited_at, archived, etc.) into the slimmer `FlowDraftValue` shape the
// global AI chat's draft store uses. Metadata stays on the runtime side —
// the draft store only holds what the AI's tools need to round-trip the
// in-flight edit.
export function flowToDraftValue(flow: Flow): FlowDraftValue {
	return {
		value: flow.value,
		schema: flow.schema ?? null,
		groups: flow.value.groups ?? null
	}
}

// Overlay a draft from the store onto an existing `Flow`. Preserves the
// metadata fields that aren't in `FlowDraftValue` (path, edited_by,
// edited_at, archived, extra_perms, …). `groups` lives inside
// `FlowValue`, so it rides along on `dv.value` automatically; the
// sibling-key on `FlowDraftValue` is purely for the AI's tool I/O.
export function applyDraftValueToFlow(flow: Flow, dv: FlowDraftValue): Flow {
	return {
		...flow,
		value: dv.value,
		schema: dv.schema ?? flow.schema
	}
}
