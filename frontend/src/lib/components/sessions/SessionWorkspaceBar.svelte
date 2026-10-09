<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		setSessionPendingWorkspace,
		type Session
	} from './sessionState.svelte'
	import ActingOnPicker from './ActingOnPicker.svelte'

	let { session }: { session: Session } = $props()

	// Effective workspace for display: committed → pending pick → active store.
	const effectiveId = $derived(getEffectiveWorkspaceId(session) ?? $workspaceStore ?? undefined)
	const pendingFork = $derived(session.pending_fork)

	function pick(id: string) {
		// Pre-send only: writes the pending pick. workspace_id stays undefined until
		// the user sends their first message. The global workspaceStore is left
		// untouched — the chat targets this pending workspace via the manager's
		// workspace resolver, so picking here must not switch the active workspace.
		setSessionPendingWorkspace(session.id, id)
	}

	function stageFork(req: { parent_workspace_id: string; id: string; name: string }) {
		setSessionPendingFork(session.id, req)
	}
</script>

<ActingOnPicker selectedId={effectiveId} {pendingFork} onPick={pick} onCreateFork={stageFork} />
