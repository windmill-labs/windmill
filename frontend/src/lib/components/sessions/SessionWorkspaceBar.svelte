<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		setSessionPendingWorkspace,
		type Session
	} from './sessionState.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'

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

<div class="flex flex-row items-center gap-1 py-0.5 px-1 text-2xs text-secondary">
	<span class="shrink-0">Acting on</span>
	<WorkspaceFamilyPicker
		selectedId={effectiveId}
		{pendingFork}
		onPick={pick}
		onCreateFork={stageFork}
		createForkCaption="Created when you send your first message."
	>
		{#snippet trigger()}
			<WorkspaceScopeTrigger workspaceId={effectiveId} {pendingFork} class="max-w-[16rem]" />
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
