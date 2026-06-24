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
	import { goto } from '$lib/navigation'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { deriveForkStatus, sessionState, type Session } from './sessionState.svelte'
	import { getRuntime } from './sessionRuntime.svelte'
	import SessionDiffDrawer from './SessionDiffDrawer.svelte'
	import SessionDiffButton from './SessionDiffButton.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { maskKey, forkDiffKindToUserDraftKind } from './modifiedItemsMask'

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

	// Draft Count from the shared Workspace Drafts resource (fetches on mount and
	// on every Server-Draft invalidation), filtered to the chat's mask when tracked.
	const drafts = useWorkspaceDrafts(() => committedId)
	const draftCount = $derived(
		mask ? drafts.items.filter((d) => mask.has(maskKey(d.kind, d.path))).length : drafts.count
	)

	// Fork comparison lives on the shared SessionRuntime cache. The status icon
	// reflects the fork's REAL relationship to its parent (ahead/diverged/in-sync).
	const comparison = $derived(runtime?.forkComparison.val)
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
	// Count for the single diff button: the deduped union of drafts and deployed-
	// ahead items (an item can be both — the unified drawer shows it once). Deduped
	// in canonical mask-key space so the count matches the drawer's rows, in both
	// the tracked (mask) and legacy (show-all) cases.
	const unifiedCount = $derived.by(() => {
		const set = new Set<string>()
		for (const d of drafts.items) {
			const k = maskKey(d.kind, d.path)
			if (!mask || mask.has(k)) set.add(k)
		}
		for (const d of comparison?.diffs ?? []) {
			if (!d.has_changes) continue
			const udk = forkDiffKindToUserDraftKind(d.kind)
			if (!udk) continue
			const k = maskKey(udk, d.path)
			if (!mask || mask.has(k)) set.add(k)
		}
		return set.size
	})

	function openReview() {
		if (!committedId || isUnavailable) return
		const params = new URLSearchParams({ workspace_id: committedId })
		// Lets the compare page read this chat's mask and preselect its items.
		if (session.chatId) params.set('from_session', session.chatId)
		// Open the draft view when there are draft-stage changes (or in a non-fork
		// session, which only has the draft view); otherwise the fork deploy view.
		if (draftCount > 0 || !isFork) params.set('mode', 'draft')
		goto(`/forks/compare?${params.toString()}`)
	}

	// Whether to render at all. Fork sessions always show the bar (it carries the
	// fork identity and surfaces the unavailable banner); non-fork sessions show
	// only when this chat has drafts to review.
	const showForkRow = $derived(
		forksAllowed && isFork && !!sessionWorkspace && !!parentWorkspace && !!parentWorkspaceId
	)
	const showDraftRow = $derived(!isFork && draftCount > 0)
</script>

{#if committedId && isUnavailable}
	<!-- Fork workspace is no longer in the user's list (deleted, archived, or
	     access revoked). The chat input is disabled by SessionWrapper while this
	     is shown. -->
	<div class="flex flex-col gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary">
		<div class="flex flex-row items-start gap-2">
			<GitPullRequestClosed class="w-4 h-4 shrink-0 text-tertiary mt-0.5" />
			<div class="flex flex-col min-w-0 flex-1">
				<span class="text-primary font-medium">The fork has been archived or deleted</span>
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
		<div class="flex items-center gap-1 shrink-0">
			<SessionDiffButton
				count={unifiedCount}
				disabled={unifiedCount === 0}
				title="View changes from this chat"
				onclick={() => diffDrawer?.open()}
			/>
			<Button variant="default" unifiedSize="xs" onclick={openReview}>Review</Button>
		</div>
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
			<span class="truncate text-secondary">
				{draftCount} draft{draftCount === 1 ? '' : 's'}
			</span>
		</div>
		<div class="flex items-center gap-1 shrink-0">
			<SessionDiffButton count={draftCount} onclick={() => diffDrawer?.open()} />
			<Button variant="default" unifiedSize="xs" onclick={openReview}>Review</Button>
		</div>
	</div>

	<SessionDiffDrawer
		bind:this={diffDrawer}
		workspaceId={committedId}
		keys={mask}
		chatId={session.chatId}
	/>
{/if}
