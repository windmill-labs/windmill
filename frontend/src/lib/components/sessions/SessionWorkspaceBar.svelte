<script lang="ts">
	import { userWorkspaces, workspaceStore, type UserWorkspace } from '$lib/stores'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		setSessionPendingWorkspace,
		syncWorkspaceTo,
		type Session
	} from './sessionState.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import { Badge } from '$lib/components/common'
	import { devBadgeText } from '$lib/utils/devWorkspaceLabel'
	import { Building, ChevronDown, GitFork } from 'lucide-svelte'

	let { session }: { session: Session } = $props()

	function findRoot(id: string | undefined, all: UserWorkspace[]): UserWorkspace | undefined {
		if (!id) return undefined
		let current = all.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = all.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}

	// Effective workspace for display: committed → pending pick → active store.
	const effectiveId = $derived(getEffectiveWorkspaceId(session) ?? $workspaceStore ?? undefined)
	const root = $derived(findRoot(effectiveId, $userWorkspaces))
	const currentWs = $derived(
		effectiveId ? $userWorkspaces.find((w) => w.id === effectiveId) : undefined
	)
	const pendingFork = $derived(session.pending_fork)

	function pick(id: string) {
		// Pre-send only: writes the pending pick. workspace_id stays
		// undefined until the user sends their first message.
		setSessionPendingWorkspace(session.id, id)
		syncWorkspaceTo(id)
	}

	function stageFork(req: { parent_workspace_id: string; id: string; name: string }) {
		setSessionPendingFork(session.id, req)
		syncWorkspaceTo(req.parent_workspace_id)
	}
</script>

<div class="flex flex-row items-center gap-1 py-0.5 px-1 text-2xs text-secondary">
	<span class="shrink-0">Run in</span>
	<WorkspaceFamilyPicker
		selectedId={effectiveId}
		{pendingFork}
		onPick={pick}
		onCreateFork={stageFork}
		createForkCaption="Created when you send your first message."
	>
		{#snippet trigger()}
			<span
				class="inline-flex flex-row items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-hover text-2xs"
			>
				{#if pendingFork || (currentWs && currentWs.id !== root?.id)}
					<GitFork class="w-3 h-3 shrink-0" />
				{:else}
					<Building class="w-3 h-3 shrink-0" />
				{/if}
				<span class="font-medium text-primary truncate max-w-[180px]">
					{pendingFork?.name ?? currentWs?.name ?? effectiveId ?? 'Pick workspace'}
				</span>
				{#if !pendingFork && currentWs?.is_dev_workspace}
					<Badge color="indigo" small>{devBadgeText(currentWs.dev_workspace_label)}</Badge>
				{/if}
				{#if pendingFork}
					<span class="text-2xs text-tertiary italic shrink-0">(new)</span>
				{/if}
				<ChevronDown class="w-3 h-3 shrink-0 text-tertiary" />
			</span>
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
