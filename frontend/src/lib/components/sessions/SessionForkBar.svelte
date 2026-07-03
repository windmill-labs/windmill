<script lang="ts">
	import {
		Archive,
		ArrowRight,
		GitCompareArrows,
		GitFork,
		GitPullRequestArrow,
		GitPullRequestClosed,
		MoveRight,
		Trash2
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import { isPremiumStore, userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	import { deriveForkStatus, sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import ForkDiffDrawer from './ForkDiffDrawer.svelte'
	import SessionDiffButton from './SessionDiffButton.svelte'

	let {
		session,
		onMove,
		onCreateForkAndMove,
		onArchive,
		onDelete
	}: {
		session: Session
		onMove?: (workspaceId: string) => void
		onCreateForkAndMove?: (fork: {
			parent_workspace_id: string
			id: string
			name: string
		}) => void | Promise<void>
		onArchive?: () => void
		onDelete?: () => void
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

	// Same gate as the sidebar WorkspaceMenu / SessionWorkspaceBar.
	// When forking isn't available the diff/review surface is moot. On cloud, forking is a
	// premium-only feature (backend caps it per paid seat).
	const forksAllowed = $derived(
		(!isCloudHosted() || $isPremiumStore) &&
			canCreateFork($userStore) &&
			$workspaceStore !== 'admins'
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

	// The committed workspace is gone, so we can't read its parent to know if
	// it was a fork — fall back to the fork-id convention for the wording.
	const committedIsFork = $derived(committedId?.startsWith('wm-fork-') ?? false)

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

{#if committedId && isUnavailable}
	<!-- Committed workspace is no longer in the user's list (deleted, archived,
	     or access revoked). Surface an actionable banner: move the session
	     to a still-valid workspace, or discard it (archive / delete). The
	     chat input is disabled by SessionWrapper while this is shown. Shown even
	     for an archived session — unarchiving in place can't help when the
	     workspace is gone, so move/discard is the only real recovery path. -->
	<div class="flex flex-col gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary">
		<div class="flex flex-row items-start gap-2">
			<GitPullRequestClosed class="w-4 h-4 shrink-0 text-tertiary mt-0.5" />
			<div class="flex flex-col min-w-0 flex-1">
				<span class="text-primary font-medium"
					>The {committedIsFork ? 'fork' : 'workspace'} has been archived or deleted</span
				>
				<span class="text-2xs text-tertiary">
					Move this session to another workspace, or discard it.
					<span class="font-mono text-tertiary" title={committedId}>{committedId}</span>
				</span>
			</div>
		</div>
		<div class="flex flex-row items-center justify-end gap-1.5">
			<WorkspaceFamilyPicker
				onPick={(workspaceId) => onMove?.(workspaceId)}
				onCreateFork={async (fork) => {
					await onCreateForkAndMove?.(fork)
				}}
				createForkCaption="Created immediately and the session moved into it."
			>
				{#snippet trigger()}
					<Button variant="default" unifiedSize="sm" startIcon={{ icon: MoveRight }}>
						Move to workspace
					</Button>
				{/snippet}
			</WorkspaceFamilyPicker>
			<Button
				variant="default"
				unifiedSize="sm"
				startIcon={{ icon: Archive }}
				dropdownItems={[{ label: 'Delete', icon: Trash2, onClick: () => onDelete?.() }]}
				onclick={() => onArchive?.()}
			>
				Archive
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
			<SessionDiffButton
				count={totalDiffs}
				disabled={totalDiffs === 0}
				onclick={() => diffDrawer?.open()}
			/>
			<Button variant="default" unifiedSize="xs" onclick={openReview}>Review</Button>
		</div>
	</div>

	<ForkDiffDrawer bind:this={diffDrawer} forkWorkspaceId={committedId} {parentWorkspaceId} />
{/if}
