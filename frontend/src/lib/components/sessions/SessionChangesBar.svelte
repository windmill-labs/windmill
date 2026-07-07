<script lang="ts">
	import {
		Archive,
		ExternalLink,
		GitPullRequestClosed,
		MoveRight,
		Pencil,
		Trash2
	} from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import { isPremiumStore, userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	import { sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import SessionDiffDrawer from './SessionDiffDrawer.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { badgeCounts, buildDeployItems } from './sessionDeployModel'
	import { useExistingMaskKeys } from './sessionDeployModel.svelte'
	import JobsSegment from '$lib/components/copilot/chat/JobsSegment.svelte'

	// Unified session bar: surfaces what the CURRENT chat changed — pending
	// drafts and deployed items — as one count badge per status that opens the
	// session diff drawer.
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

	// Same gate as the sidebar WorkspaceMenu / SessionWorkspaceBar. On cloud,
	// forking is a premium-only feature (backend caps it per paid seat).
	const forksAllowed = $derived(
		(!isCloudHosted() || $isPremiumStore) &&
			canCreateFork($userStore) &&
			$workspaceStore !== 'admins'
	)

	const runtime = $derived(getRuntime(session.id))

	// The chat's modified-items mask (`${UserDraftItemKind}:${storagePath}`).
	// undefined = legacy/untracked chat → fall back to showing every draft.
	// A Set (even empty) → filter to just this chat's items.
	const mask = $derived(runtime?.manager.modifiedItems)

	// Workspace Drafts (fetches on mount and on every Server-Draft invalidation);
	// scoped to the chat's mask inside the deploy model below.
	const drafts = useWorkspaceDrafts(() => committedId)

	// The committed workspace vanished from the user's list (deleted, archived,
	// or access revoked) — the session can't operate in it anymore.
	const isUnavailable = $derived(!!committedId && !sessionWorkspace)

	// The committed workspace is gone, so we can't read its parent to know if
	// it was a fork — fall back to the fork-id convention for the wording.
	const committedIsFork = $derived(committedId?.startsWith('wm-fork-') ?? false)

	// Full dock refresh: re-run existence checks + draft list. Both are
	// stale-while-revalidate — a hard reset would blank dockItems for the
	// round-trip, hiding the bar (and unmounting the drawer) mid-deploy.
	function refreshDock() {
		existing.refresh()
		drafts.refresh()
	}

	// Refresh the draft list when the AI finishes a turn (loading true → false):
	// tool calls may have created/edited/deleted items. Deploys happen
	// server-side, so the frontend only has this coarse signal.
	let wasLoading = $state(false)
	$effect(() => {
		const isLoading = runtime?.manager.loading ?? false
		if (wasLoading && !isLoading) refreshDock()
		wasLoading = isLoading
	})

	// Refresh when the user comes back to this view — covers edits made
	// elsewhere while we were away. Two signals: visibilitychange (another tab
	// in the same window) and window focus (a second browser window, where this
	// tab never goes hidden so visibilitychange never fires — e.g. deploying
	// from the full-page editor side by side).
	$effect(() => {
		if (!committedId) return
		function refreshIfCurrent() {
			if (document.visibilityState !== 'visible') return
			if (sessionState.currentSessionId !== session.id) return
			refreshDock()
		}
		document.addEventListener('visibilitychange', refreshIfCurrent)
		window.addEventListener('focus', refreshIfCurrent)
		return () => {
			document.removeEventListener('visibilitychange', refreshIfCurrent)
			window.removeEventListener('focus', refreshIfCurrent)
		}
	})

	let diffDrawer: SessionDiffDrawer | undefined = $state(undefined)

	// Opening the drawer re-fetches its own data, so it can show fresher state
	// than the badge just clicked (e.g. "1 draft" that was deployed elsewhere in
	// the meantime). Re-sync the dock alongside so the two never disagree.
	function openDrawer() {
		refreshDock()
		diffDrawer?.open()
	}

	// The dock counts are computed from the same pure model over this chat's
	// drafts; the readout mirrors the drawer's item states.
	// Existence check for mask-only (deployed) items — without it they'd be
	// dropped from the dock counts while the drawer (which runs the same check)
	// still shows them as Deployed.
	const existing = useExistingMaskKeys(() => ({
		draftItems: drafts.items,
		mask: committedId ? mask : undefined,
		workspaceId: committedId ?? ''
	}))
	const dockItems = $derived(
		committedId
			? buildDeployItems({ draftItems: drafts.items, mask, existingKeys: existing.keys })
			: []
	)
	const dockCounts = $derived(badgeCounts(dockItems))

	// Deletion-only fork chat: the mask has entries but every one failed the
	// (resolved) existence check — the chat's edits were deletions, which have no
	// dock row by design. The pending fork→parent removal is still reviewable on
	// the compare page, so the bar must keep that doorway instead of vanishing.
	// Gated on a resolved check (undefined = still loading) so the bar doesn't
	// flash this state while deployed rows are being confirmed, and on isFork —
	// a non-fork deletion is immediate and final, with nothing left to review.
	const deletionOnly = $derived(
		isFork && (mask?.size ?? 0) > 0 && existing.keys !== undefined && dockItems.length === 0
	)
	const compareHref = $derived(
		committedId
			? `/forks/compare?workspace_id=${encodeURIComponent(committedId)}&mode=fork` +
					(session.chatId ? `&from_session=${encodeURIComponent(session.chatId)}` : '')
			: undefined
	)

	// One "Edits" bar for both fork and non-fork sessions, shown when this chat
	// edited anything. The fork's identity lives in the modal (SessionDiffDrawer's
	// title), not on the bar. Fork sessions still need the forking gate + a
	// resolvable fork/parent pair.
	const showBar = $derived(
		!!committedId &&
			(dockItems.length > 0 || deletionOnly) &&
			(!isFork || (forksAllowed && !!sessionWorkspace && !!parentWorkspace && !!parentWorkspaceId))
	)

	// Background jobs the chat started (rendered as the Jobs segment). The session
	// bar shows if there are edits OR jobs; each segment hides when its side is empty.
	const hasJobs = $derived((runtime?.manager.backgroundJobs.length ?? 0) > 0)
</script>

{#snippet dock()}
	<!-- One count badge per row status, same vocabulary/colors as the drawer's
	     badges (draft/deployed) so the bar reads at a glance. Display-only: the
	     enclosing Edits segment button opens the drawer. -->
	<div class="flex items-center gap-1 shrink-0">
		{#if dockCounts.draft > 0}
			<Badge small color="indigo">
				{dockCounts.draft} draft{dockCounts.draft === 1 ? '' : 's'}
			</Badge>
		{/if}
		{#if dockCounts.deployed > 0}
			<Badge small color="green">
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
{:else if committedId && (showBar || hasJobs)}
	<!-- Segmented session bar: an Edits segment (what the AI changed this session)
	     and a Jobs segment (background jobs it started), sharing one border box.
	     The Edits segment (or, in its absence, a flex-1 spacer) fills the left so the
	     Jobs segment stays pinned to the right whether or not there are edits. Fork
	     identity / sync status lives inside the modal. -->
	<div
		class="flex h-[38px] items-stretch overflow-hidden rounded-md border bg-surface-tertiary text-xs"
	>
		{#if showBar}
			{#if deletionOnly && compareHref}
				<!-- Deleted items have no drawer row; the pending fork→parent removal is
				     reviewed on the compare page, so the segment links there instead. -->
				<a
					href={compareHref}
					target="_blank"
					rel="noopener noreferrer"
					title="This chat's edits were deletions — review and promote them on the compare page"
					class="flex min-w-0 flex-1 items-center gap-1.5 px-3 hover:bg-surface-hover"
				>
					<Pencil class="h-3.5 w-3.5 shrink-0 text-secondary" />
					<span class="shrink-0 font-normal text-primary">Edits</span>
					<span
						class="ml-auto inline-flex items-center gap-1 truncate text-2xs font-medium text-accent"
					>
						Review deletions <ExternalLink class="h-3 w-3 shrink-0" />
					</span>
				</a>
			{:else}
				<button
					type="button"
					class="flex min-w-0 flex-1 items-center gap-1.5 px-3 hover:bg-surface-hover"
					title="Edited by the chat during this session"
					onclick={openDrawer}
				>
					<Pencil class="h-3.5 w-3.5 shrink-0 text-secondary" />
					<span class="shrink-0 font-normal text-primary">Edits</span>
					<span class="ml-auto">{@render dock()}</span>
				</button>
			{/if}
		{:else}
			<!-- No edits: a spacer fills the left so the Jobs segment stays right-aligned. -->
			<div class="flex-1"></div>
		{/if}
		{#if hasJobs}
			<!-- standalone={false}: renders just the chip trigger as a full-height
			     segment; the border box above provides the chrome. -->
			<JobsSegment />
		{/if}
	</div>
{/if}

<!-- Mounted independently of the bar: a transient dip of the dock counts to
     zero (mid-refresh after a deploy) must not unmount an open drawer. -->
{#if committedId && !isUnavailable}
	<SessionDiffDrawer
		bind:this={diffDrawer}
		workspaceId={committedId}
		{parentWorkspaceId}
		chatId={session.chatId}
		keys={mask}
		onDataChanged={refreshDock}
		onItemDeployed={(item) =>
			void runtime?.manager.renameModifiedItem(item.draftKind, item.path, item.displayPath)}
		onItemDiscarded={(item) => void runtime?.manager.removeModifiedItem(item.draftKind, item.path)}
	/>
{/if}
