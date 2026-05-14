<script lang="ts">
	import { ArrowRight, GitCompareArrows, GitFork, GitMerge } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { userWorkspaces } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import type { Session, SessionTarget } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import ForkDiffDrawer from './ForkDiffDrawer.svelte'
	import ForkDiffCountDropdown from './ForkDiffCountDropdown.svelte'

	let {
		session,
		onOpenInPanel
	}: {
		session: Session
		onOpenInPanel?: (target: SessionTarget, summary?: string) => void
	} = $props()

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
	const diffs = $derived(comparison?.diffs ?? [])

	$effect(() => {
		if (!runtime || !committedId || !parentWorkspaceId) return
		void runtime.ensureForkComparison(parentWorkspaceId, committedId)
	})

	export function refresh() {
		if (runtime && committedId && parentWorkspaceId) {
			runtime.invalidateForkComparison()
			void runtime.ensureForkComparison(parentWorkspaceId, committedId)
		}
	}

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
			<ForkDiffCountDropdown {diffs} workspaceId={committedId} {onOpenInPanel} />
			<Button
				variant="subtle"
				unifiedSize="xs"
				startIcon={{ icon: GitCompareArrows }}
				disabled={totalDiffs === 0}
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

	<ForkDiffDrawer bind:this={diffDrawer} forkWorkspaceId={committedId} {parentWorkspaceId} />
{/if}
