<script lang="ts">
	import { GitFork, GitCompareArrows, GitMerge, ArrowRight } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { userWorkspaces } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import type { Session } from './sessionState.svelte'
	import ForkDiffDrawer from './ForkDiffDrawer.svelte'

	let { session }: { session: Session } = $props()

	// Resolve the session's workspace and its parent (root) from the user's
	// workspace list. The bar only renders when both are present.
	const sessionWorkspace = $derived($userWorkspaces.find((w) => w.id === session.workspace_id))
	const parentWorkspaceId = $derived(sessionWorkspace?.parent_workspace_id ?? undefined)
	const parentWorkspace = $derived(
		parentWorkspaceId ? $userWorkspaces.find((w) => w.id === parentWorkspaceId) : undefined
	)
	const isFork = $derived(!!parentWorkspaceId)

	let diffDrawer: ForkDiffDrawer | undefined = $state(undefined)

	function openReview() {
		goto(`/forks/compare?workspace_id=${encodeURIComponent(session.workspace_id)}`)
	}
</script>

{#if isFork && sessionWorkspace && parentWorkspace && parentWorkspaceId}
	<div
		class="flex flex-row items-center justify-between gap-2 px-3 py-1.5 text-xs border-b border-light bg-surface-secondary"
	>
		<div class="flex items-center gap-1.5 min-w-0">
			<GitFork class="w-3.5 h-3.5 shrink-0 text-secondary" />
			<span class="truncate text-secondary" title={sessionWorkspace.name}>
				{sessionWorkspace.name}
			</span>
			<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
			<span class="truncate text-secondary" title={parentWorkspace.name}>
				{parentWorkspace.name}
			</span>
		</div>
		<div class="flex items-center gap-1 shrink-0">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: GitCompareArrows }}
				on:click={() => diffDrawer?.open()}
			>
				Diff
			</Button>
			<Button
				variant="default"
				unifiedSize="xs"
				startIcon={{ icon: GitMerge }}
				on:click={openReview}
			>
				Review
			</Button>
		</div>
	</div>

	<ForkDiffDrawer
		bind:this={diffDrawer}
		forkWorkspaceId={session.workspace_id}
		{parentWorkspaceId}
	/>
{/if}
