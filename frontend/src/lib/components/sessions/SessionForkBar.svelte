<script lang="ts">
	import { ArrowRight, GitCompareArrows, GitFork, GitMerge } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { userWorkspaces } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import ForkDiffDrawer from './ForkDiffDrawer.svelte'

	let { session }: { session: Session } = $props()

	// The fork bar surfaces a committed workspace relationship — only
	// visible after the session locked its workspace at first send. Drafts
	// (workspace_id undefined) get nothing here.
	const committedId = $derived(session.workspace_id)
	const sessionWorkspace = $derived(
		committedId ? $userWorkspaces.find((w) => w.id === committedId) : undefined
	)
	const parentWorkspaceId = $derived(sessionWorkspace?.parent_workspace_id ?? undefined)
	const parentWorkspace = $derived(
		parentWorkspaceId ? $userWorkspaces.find((w) => w.id === parentWorkspaceId) : undefined
	)
	const isFork = $derived(!!parentWorkspaceId)

	let diffDrawer: ForkDiffDrawer | undefined = $state(undefined)

	// Comparison data lives on the shared SessionRuntime resource so any
	// future consumer (e.g. the diff drawer, a merge action) reads the
	// same cache and can invalidate it after mutating the fork.
	const runtime = $derived(getRuntime(session.id))
	const comparison = $derived(runtime?.forkComparison.val)
	const totalDiffs = $derived(comparison?.summary?.total_diffs ?? 0)

	$effect(() => {
		if (!runtime || !committedId || !parentWorkspaceId) return
		void runtime.ensureForkComparison(parentWorkspaceId, committedId)
	})

	function refreshComparison() {
		if (!runtime || !committedId || !parentWorkspaceId) return
		runtime.invalidateForkComparison()
		void runtime.ensureForkComparison(parentWorkspaceId, committedId)
	}

	// Refresh when the AI finishes a turn (loading transitions true →
	// false). Tool calls in that turn may have created / edited / deleted
	// fork items, so the diff count needs to reflect them immediately.
	// Cheap over-refresh on read-only turns is fine — single API call.
	let wasLoading = $state(false)
	$effect(() => {
		const isLoading = runtime?.manager.loading ?? false
		if (wasLoading && !isLoading) refreshComparison()
		wasLoading = isLoading
	})

	// Refresh when the tab regains visibility — covers edits made in
	// another tab or by another user while we were away. No polling
	// while the tab is hidden or idle.
	$effect(() => {
		if (!runtime || !committedId || !parentWorkspaceId) return
		function onVisibilityChange() {
			if (document.visibilityState !== 'visible') return
			if (sessionState.currentSessionId !== session.id) return
			refreshComparison()
		}
		document.addEventListener('visibilitychange', onVisibilityChange)
		return () => document.removeEventListener('visibilitychange', onVisibilityChange)
	})

	export const refresh = refreshComparison

	function openReview() {
		if (!committedId) return
		goto(`/forks/compare?workspace_id=${encodeURIComponent(committedId)}`)
	}
</script>

{#if isFork && sessionWorkspace && parentWorkspace && parentWorkspaceId && committedId}
	<div
		class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
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
				disabled={totalDiffs === 0}
				title="{totalDiffs} modified item{totalDiffs === 1 ? '' : 's'}"
				on:click={() => diffDrawer?.open()}
			>
				{totalDiffs}
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

	<ForkDiffDrawer bind:this={diffDrawer} forkWorkspaceId={committedId} {parentWorkspaceId} />
{/if}
