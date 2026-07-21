<script lang="ts">
	import { Archive, ExternalLink, GitPullRequestClosed, MoveRight, Trash2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import SessionStatusPopover from './SessionStatusPopover.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import { isPremiumStore, userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	import { sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import SessionDiffDrawer from './SessionDiffDrawer.svelte'
	import { TOKEN_TRIGGER_CLASS } from './SessionStatusToken.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { badgeCounts, badgeOf, buildDeployItems } from './sessionDeployModel'
	import { useExistingMaskKeys } from './sessionDeployModel.svelte'
	import JobsSegment from '$lib/components/copilot/chat/JobsSegment.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import ArtifactsSegment from '$lib/components/copilot/chat/artifacts/ArtifactsSegment.svelte'

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
	function openDrawer(focusKey?: string) {
		refreshDock()
		diffDrawer?.open(focusKey)
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
	const hasArtifacts = $derived((runtime?.manager.artifacts.artifacts.length ?? 0) > 0)

	// Drafts are what still needs action, so the token counts only them while any
	// are pending; once none are left it turns green and counts the deployed.
	const editsLabel = $derived(
		dockCounts.draft > 0
			? `${dockCounts.draft} draft${dockCounts.draft === 1 ? '' : 's'}`
			: `${dockCounts.deployed} deployed`
	)
	let editsOpen = $state(false)

	// Only draft-vs-deployed drives the color: stale/failed live in the drawer's
	// deploy model, not here.
	const editsColorClass = $derived(
		dockCounts.draft > 0
			? '!bg-indigo-100 !text-indigo-800 hover:!bg-indigo-200 dark:!bg-indigo-700/40 dark:!text-indigo-100 dark:hover:!bg-indigo-600/40'
			: '!bg-green-100 !text-green-700 hover:!bg-green-200 dark:!bg-green-700/40 dark:!text-green-100 dark:hover:!bg-green-600/40'
	)
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
{:else if committedId && (showBar || hasArtifacts || hasJobs)}
	<div class="flex items-center gap-3 rounded-md border bg-surface-tertiary px-2 py-2 text-xs">
		{#if showBar}
			{#if deletionOnly && compareHref}
				<!-- Deleted items have no drawer row, so this token links to the compare
				     page (where the fork→parent removal is reviewed) rather than opening
				     a popover. -->
				<a
					href={compareHref}
					target="_blank"
					rel="noopener noreferrer"
					title="This chat's edits were deletions — review and promote them on the compare page"
					class={TOKEN_TRIGGER_CLASS}
				>
					<span class="truncate font-normal text-secondary">Edits</span>
					<ExternalLink class="h-3 w-3 shrink-0 text-tertiary" />
				</a>
			{:else}
				<SessionStatusPopover
					bind:open={editsOpen}
					label={editsLabel}
					title="Edited this session"
					items={dockItems}
					itemKey={(item) => item.key}
					rowTitle={(item) => item.displayPath}
					onPick={(item) => openDrawer(item.key)}
					triggerClass={`${TOKEN_TRIGGER_CLASS} ${editsColorClass}`}
					widthClass="w-96"
					maxHeightClass="max-h-[min(9rem,50vh)]"
				>
					{#snippet row(item)}
						<RowIcon kind={item.deployKind} path={item.path} size={14} />
						<span class="min-w-0 flex-1 truncate font-mono font-normal text-primary">
							{item.displayPath}
						</span>
						{#if badgeOf(item) === 'draft'}
							<Badge small color="indigo">
								{item.draftOnly ? 'Draft only' : 'Draft'}
							</Badge>
						{:else}
							<Badge small color="green">Deployed</Badge>
						{/if}
					{/snippet}
				</SessionStatusPopover>
			{/if}
		{/if}
		{#if hasArtifacts}
			<ArtifactsSegment />
		{/if}
		{#if hasJobs}
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
