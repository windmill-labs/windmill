<script lang="ts">
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'
	import type { PendingFork } from './sessionState.svelte'

	let {
		selectedId,
		pendingFork,
		onPick,
		onCreateFork
	}: {
		selectedId: string | undefined
		pendingFork: PendingFork | undefined
		onPick: (workspaceId: string) => void
		onCreateFork: (fork: PendingFork) => void
	} = $props()
</script>

<!-- Where a session's first message will act. A fork picked here is only staged: it is
     created when that message is sent. -->
<div class="flex flex-row items-center gap-1 py-0.5 px-1 text-2xs text-secondary">
	<span class="shrink-0">Acting on</span>
	<WorkspaceFamilyPicker
		{selectedId}
		{pendingFork}
		{onPick}
		{onCreateFork}
		createForkCaption="Created when you send your first message."
	>
		{#snippet trigger()}
			<WorkspaceScopeTrigger workspaceId={selectedId} {pendingFork} class="max-w-[16rem]" />
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
