<script lang="ts">
	import {
		ArrowRight,
		GitCompareArrows,
		GitFork,
		GitMerge,
		GitPullRequestArrow,
		GitPullRequestClosed
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { deriveForkStatus, sessionState, type Session } from './sessionState.svelte'
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

	// Same gate as the sidebar WorkspaceMenu / SessionWorkspaceBar.
	// When forking isn't available the diff/review surface is moot.
	const forksAllowed = $derived(
		!isCloudHosted() && !isRuleActive('DisableWorkspaceForking') && $workspaceStore !== 'admins'
	)

	let diffDrawer: ForkDiffDrawer | undefined = $state(undefined)

	// Comparison data lives on the shared SessionRuntime resource so any
	// future consumer (e.g. the diff drawer, a merge action) reads the
	// same cache and can invalidate it after mutating the fork.
	const runtime = $derived(getRuntime(session.id))
	const comparison = $derived(runtime?.forkComparison.val)
	const totalDiffs = $derived(comparison?.summary?.total_diffs ?? 0)
	const forkStatus = $derived(deriveForkStatus(session, $userWorkspaces, comparison))
	const isUnavailable = $derived(forkStatus === 'unavailable')

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
	let wasLoading = $state(false)
	$effect(() => {
		const isLoading = runtime?.manager.loading ?? false
		if (wasLoading && !isLoading) refreshComparison()
		wasLoading = isLoading
	})

	// Refresh when the tab regains visibility — covers edits made in
	// another tab or by another user while we were away.
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
		if (!committedId || isUnavailable) return
		goto(`/forks/compare?workspace_id=${encodeURIComponent(committedId)}`)
	}
</script>

{#if forksAllowed && committedId && isUnavailable}
	<!-- Fork workspace is no longer in the user's list (deleted, archived,
	     or access revoked). Surface a terminal read-only banner; we don't
	     know the parent so the → label is omitted. -->
	<div
		class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
	>
		<div class="flex items-center gap-1.5 min-w-0">
			<GitPullRequestClosed class="w-3.5 h-3.5 shrink-0 text-tertiary" />
			<span class="truncate text-tertiary line-through" title={committedId}>
				{committedId}
			</span>
			<span class="text-2xs text-tertiary italic shrink-0">unavailable</span>
		</div>
		<div class="flex items-center gap-1 shrink-0">
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: GitCompareArrows }}
				disabled
				title="Fork is unavailable"
			>
				0
			</Button>
			<Button
				variant="default"
				unifiedSize="xs"
				startIcon={{ icon: GitMerge }}
				disabled
				title="Fork is unavailable"
			>
				Review
			</Button>
		</div>
	</div>
{:else if forksAllowed && isFork && sessionWorkspace && parentWorkspace && parentWorkspaceId && committedId}
	{@const StatusIcon =
		forkStatus === 'ahead'
			? GitPullRequestArrow
			: forkStatus === 'diverged'
				? GitCompareArrows
				: GitFork}
	{@const statusColor =
		forkStatus === 'ahead'
			? 'text-blue-500'
			: forkStatus === 'diverged'
				? 'text-amber-500'
				: 'text-secondary'}
	{@const statusTitle =
		forkStatus === 'ahead'
			? 'Ahead of parent'
			: forkStatus === 'diverged'
				? 'Diverged from parent'
				: forkStatus === 'in_sync'
					? 'In sync with parent'
					: 'Fork'}
	<div
		class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
	>
		<div class="flex items-center gap-1.5 min-w-0">
			<span title={statusTitle} class="inline-flex shrink-0">
				<StatusIcon class="w-3.5 h-3.5 {statusColor}" />
			</span>
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
