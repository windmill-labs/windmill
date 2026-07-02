<script lang="ts">
	import { Archive, GitPullRequestClosed, MoveRight, Pencil, Trash2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { deriveForkStatus, sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import SessionDiffDrawer from './SessionDiffDrawer.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { badgeCounts, buildDeployItems, type SessionContext } from './sessionDeployModel'
	import { useExistingMaskKeys } from './sessionDeployModel.svelte'

	// Unified session bar: replaces the separate draft + fork bars. It surfaces
	// what the CURRENT chat changed — drafts it wrote and (in a fork) items it
	// deployed ahead of the parent — as one count badge per status (drafts /
	// ahead / conflicts / deployed) that all open the session diff drawer.
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
		// A fork *deletion* only lands in the comparison via an async backend tally
		// (~300ms after the turn), and produces no draft — so the immediate fetch
		// above races it. Re-poll (700ms/2200ms) so deletions surface reliably.
		runtime.scheduleForkComparisonRefresh()
	}

	// Full dock refresh: fresh existence checks + draft list + fork comparison.
	function refreshDock() {
		existing.reset()
		drafts.refresh()
		refreshComparison()
	}

	// Refresh both the draft list and the fork comparison when the AI finishes a
	// turn (loading true → false): tool calls may have created/edited/deleted
	// items. Deploys happen server-side, so the frontend only has this coarse signal.
	let wasLoading = $state(false)
	$effect(() => {
		const isLoading = runtime?.manager.loading ?? false
		if (wasLoading && !isLoading) refreshDock()
		wasLoading = isLoading
	})

	// Refresh when the tab regains visibility — covers edits in another tab / by
	// another user while we were away.
	$effect(() => {
		if (!committedId) return
		function onVisibilityChange() {
			if (document.visibilityState !== 'visible') return
			if (sessionState.currentSessionId !== session.id) return
			refreshDock()
		}
		document.addEventListener('visibilitychange', onVisibilityChange)
		return () => document.removeEventListener('visibilitychange', onVisibilityChange)
	})

	let diffDrawer: SessionDiffDrawer | undefined = $state(undefined)

	// The dock counts are computed from the same pure model over this chat's drafts
	// + fork comparison; the readout mirrors the drawer's item states.
	const context = $derived<SessionContext>({
		isFork,
		currentWorkspaceId: committedId ?? '',
		parentWorkspaceId,
		parentName: parentWorkspace?.name ?? parentWorkspaceId
	})
	// Existence check for mask-only (deployed) items — without it they'd be
	// dropped from the dock counts while the drawer (which runs the same check)
	// still shows them as Deployed.
	const existing = useExistingMaskKeys(() => ({
		draftItems: drafts.items,
		comparison,
		mask: committedId ? mask : undefined,
		context,
		workspaceId: committedId ?? ''
	}))
	const dockItems = $derived(
		committedId
			? buildDeployItems({
					draftItems: drafts.items,
					comparison,
					mask,
					existingKeys: existing.keys,
					context
				})
			: []
	)
	const dockCounts = $derived(badgeCounts(dockItems))

	// One "Edits" bar for both fork and non-fork sessions, shown when this chat
	// edited anything. The fork's own name / sync status no longer lives on the bar
	// — it's inside the modal (SessionDiffDrawer's title). Fork sessions still need
	// the forking gate + a resolvable fork/parent pair.
	const showBar = $derived(
		!!committedId &&
			dockItems.length > 0 &&
			(!isFork || (forksAllowed && !!sessionWorkspace && !!parentWorkspace && !!parentWorkspaceId))
	)
</script>

{#snippet dock()}
	<!-- One count badge per row status, same vocabulary/colors as the drawer's
	     badges (draft/ahead/conflict/deployed) so the bar reads at a glance. -->
	<div class="flex items-center gap-1 shrink-0">
		{#if dockCounts.draft > 0}
			<Badge small clickable color="indigo" onclick={() => diffDrawer?.open()}>
				{dockCounts.draft} draft{dockCounts.draft === 1 ? '' : 's'}
			</Badge>
		{/if}
		{#if dockCounts.ahead > 0}
			<Badge small clickable color="green" onclick={() => diffDrawer?.open()}>
				{dockCounts.ahead} ahead
			</Badge>
		{/if}
		{#if dockCounts.conflict > 0}
			<Badge small clickable color="red" onclick={() => diffDrawer?.open()}>
				{dockCounts.conflict} conflict{dockCounts.conflict === 1 ? '' : 's'}
			</Badge>
		{/if}
		{#if dockCounts.deployed > 0}
			<Badge small clickable color="violet" onclick={() => diffDrawer?.open()}>
				{dockCounts.deployed} deployed
			</Badge>
		{/if}
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
{:else if showBar && committedId}
	<!-- The "Edits" bar: what the AI changed this session. Fork identity / sync
	     status lives inside the modal, not here. -->
	<div
		class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
	>
		<div class="flex items-center gap-1.5 min-w-0" title="Edited by the chat during this session">
			<Pencil class="w-3.5 h-3.5 shrink-0 text-secondary" />
			<span class="truncate text-secondary font-medium">Edits</span>
		</div>
		{@render dock()}
	</div>

	<SessionDiffDrawer
		bind:this={diffDrawer}
		workspaceId={committedId}
		{parentWorkspaceId}
		chatId={session.chatId}
		keys={mask}
		onDataChanged={refreshDock}
	/>
{/if}
