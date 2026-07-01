<script lang="ts">
	import {
		Archive,
		ArrowRight,
		GitCompareArrows,
		GitFork,
		GitPullRequestArrow,
		GitPullRequestClosed,
		MoveRight,
		Pencil,
		Trash2
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { deriveForkStatus, sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import SessionDiffDrawer from './SessionDiffDrawer.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { buildDeployItems, segmentCounts, type SessionContext } from './sessionDeployModel'

	// Unified session bar: replaces the separate draft + fork bars. It surfaces
	// what the CURRENT chat changed — drafts it wrote and (in a fork) items it
	// deployed ahead of the parent — and routes a single Review button to the
	// compare page with those items preselected.
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

	// Only meaningful once the session committed to a workspace (post first send).
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
	const forksAllowed = $derived(
		!isCloudHosted() && !isRuleActive('DisableWorkspaceForking') && $workspaceStore !== 'admins'
	)

	const runtime = $derived(getRuntime(session.id))

	// The chat's modified-items mask (`${UserDraftItemKind}:${storagePath}`).
	// undefined = legacy/untracked chat → fall back to showing every draft/diff
	// (old behavior). A Set (even empty) → filter to just this chat's items.
	const mask = $derived(runtime?.manager.modifiedItems)

	// Workspace Drafts (fetches on mount and on every Server-Draft invalidation);
	// scoped to the chat's mask inside the deploy model below.
	const drafts = useWorkspaceDrafts(() => committedId)

	// Fork comparison lives on the shared SessionRuntime cache. The status icon
	// reflects the fork's REAL relationship to its parent (ahead/diverged/in-sync).
	const comparison = $derived(runtime?.forkComparison.val)
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

	// Refresh both the draft list and the fork comparison when the AI finishes a
	// turn (loading true → false): tool calls may have created/edited/deleted
	// items. Deploys happen server-side, so the frontend only has this coarse signal.
	let wasLoading = $state(false)
	$effect(() => {
		const isLoading = runtime?.manager.loading ?? false
		if (wasLoading && !isLoading) {
			drafts.refresh()
			refreshComparison()
		}
		wasLoading = isLoading
	})

	// Refresh when the tab regains visibility — covers edits in another tab / by
	// another user while we were away.
	$effect(() => {
		if (!committedId) return
		function onVisibilityChange() {
			if (document.visibilityState !== 'visible') return
			if (sessionState.currentSessionId !== session.id) return
			drafts.refresh()
			refreshComparison()
		}
		document.addEventListener('visibilitychange', onVisibilityChange)
		return () => document.removeEventListener('visibilitychange', onVisibilityChange)
	})

	let diffDrawer: SessionDiffDrawer | undefined = $state(undefined)

	// The dock counts mirror the drawer's lifecycle segments — computed from the
	// same pure model over this chat's drafts + fork comparison, so a count button
	// opens the drawer preset to exactly that filter.
	const context = $derived<SessionContext>({
		isFork,
		currentWorkspaceId: committedId ?? '',
		parentWorkspaceId,
		parentName: parentWorkspace?.name ?? parentWorkspaceId
	})
	const dockItems = $derived(
		committedId ? buildDeployItems({ draftItems: drafts.items, comparison, mask, context }) : []
	)
	const dockCounts = $derived(segmentCounts(dockItems))

	// Whether to render at all. Fork sessions always show the bar (it carries the
	// fork identity and surfaces the unavailable banner); non-fork sessions show
	// only when this chat has something to review.
	const showForkRow = $derived(
		forksAllowed && isFork && !!sessionWorkspace && !!parentWorkspace && !!parentWorkspaceId
	)
	const showDraftRow = $derived(!isFork && dockCounts.to_review > 0)
</script>

{#snippet dock()}
	<div class="flex items-center gap-1 shrink-0">
		<button
			type="button"
			disabled={dockCounts.to_review === 0}
			onclick={() => diffDrawer?.open('to_review')}
			class="text-2xs px-2 py-1 rounded border transition-colors {dockCounts.to_review > 0
				? 'border-blue-300 text-blue-700 hover:bg-surface-hover dark:border-blue-500/40 dark:text-blue-300'
				: 'border-light text-tertiary cursor-default'}"
		>
			{dockCounts.to_review} to review
		</button>
		{#if dockCounts.done > 0}
			<button
				type="button"
				onclick={() => diffDrawer?.open('done')}
				class="text-2xs px-2 py-1 rounded border border-light text-secondary hover:bg-surface-hover transition-colors"
			>
				{dockCounts.done} done
			</button>
		{/if}
		<Button variant="default" unifiedSize="xs" onclick={() => diffDrawer?.open('to_review')}>
			Review
		</Button>
	</div>
{/snippet}

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
{:else if showForkRow && parentWorkspaceId && committedId}
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
			<span class="truncate text-secondary" title={sessionWorkspace?.name}>
				{sessionWorkspace?.name}
			</span>
			<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
			<span class="truncate text-secondary" title={parentWorkspace?.name}>
				{parentWorkspace?.name}
			</span>
		</div>
		{@render dock()}
	</div>

	<SessionDiffDrawer
		bind:this={diffDrawer}
		workspaceId={committedId}
		{parentWorkspaceId}
		keys={mask}
		chatId={session.chatId}
	/>
{:else if showDraftRow && committedId}
	<div
		class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
	>
		<div class="flex items-center gap-1.5 min-w-0">
			<span class="inline-flex shrink-0"><Pencil class="w-3.5 h-3.5 text-secondary" /></span>
			<span class="truncate text-secondary">Changes</span>
		</div>
		{@render dock()}
	</div>

	<SessionDiffDrawer
		bind:this={diffDrawer}
		workspaceId={committedId}
		keys={mask}
		chatId={session.chatId}
	/>
{/if}
