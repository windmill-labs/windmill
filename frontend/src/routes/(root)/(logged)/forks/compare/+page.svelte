<script lang="ts">
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import CompareWorkspaces from '$lib/components/CompareWorkspaces.svelte'
	import CompareDrafts from '$lib/components/CompareDrafts.svelte'
	import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'
	import { fetchWorkspaceComparison, invalidateWorkspaceComparison } from '$lib/workspaceComparison'
	import CompareTargetPicker from '$lib/components/CompareTargetPicker.svelte'
	import {
		archiveSessionsForWorkspace,
		deleteSessionsForWorkspace,
		reconcileAfterWorkspaceChange
	} from '$lib/components/sessions/sessionState.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { diffActionableInDirection } from '$lib/utils_workspace_deploy'
	import { page } from '$app/state'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { onDestroy, untrack } from 'svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Archive, Trash2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'
	import { readChatModifiedItems } from '$lib/components/copilot/chat/HistoryManager.svelte'
	import {
		COMPARE_ITEMS_PARAM,
		maskHasDraftRow,
		parseItemsMaskParam
	} from '$lib/components/sessions/modifiedItemsMask'

	type CompareMode = 'fork' | 'draft'

	/** The last comparison that came back, with the pair it describes. Kept together: a comparison
	 *  is only an answer about the two workspaces it was asked about. */
	let comparisonResult = $state<{ pair: string; value: WorkspaceComparison } | undefined>(undefined)

	// Which workspace this page compares, read rather than followed. The URL answers it when it
	// names one: `?workspace=` is a switch already made, which the root layout applies to the
	// store, and `?workspace_id=` is the workspace a link asked to compare — a session's Review
	// button passes the fork it committed to while deliberately leaving the navigation workspace
	// alone (SessionChangesBar), as does the prod→dev link in UpdateDevWorkspaceModal, so it has
	// to outrank the store. With neither param the store is the answer, and a switch in the trail
	// moves the page because `fixupUrlAfterWorkspaceSwitch` rewrites whichever param is there.
	const currentWorkspaceId = $derived(
		page.url.searchParams.get('workspace') ??
			page.url.searchParams.get('workspace_id') ??
			$workspaceStore ??
			undefined
	)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === currentWorkspaceId))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)

	// `?target=` overrides the destination with an arbitrary workspace, for the
	// one-off migration the lineage cannot express. It is one-way (current →
	// target): nothing tallies such a pair, so a cold diff has no deploy history
	// telling which side a change came from.
	//
	// Read whatever the URL holds, including on a page opened with no workspace param at all —
	// which is how the migration starts, from "Merge into another workspace" in the workspace
	// settings, and where the picker below is the only thing that sets this.
	const targetParam = $derived(page.url.searchParams.get('target') ?? undefined)
	const compareTargetId = $derived(targetParam ?? parentWorkspaceId ?? undefined)
	/** The pair on screen, as one value: what a comparison has to describe to be about it. */
	const pairKey = $derived(`${currentWorkspaceId}->${compareTargetId}`)
	/** The comparison, but only while it describes the pair on screen. A switch changes the pair
	 *  during render and the card is remounted in that same render, so a comparison handed over
	 *  without this check would be the previous pair's — and the card would fetch that pair's item
	 *  paths from the new workspace, where every one of them is a 404 by construction. */
	const comparison = $derived(
		comparisonResult?.pair === pairKey ? comparisonResult.value : undefined
	)
	const isArbitraryTarget = $derived(!!compareTargetId && compareTargetId !== parentWorkspaceId)
	// Fork/dev workspaces are identified by their parent link, not the `wm-fork-` id
	// prefix. Distinct from having a compare target: a root workspace has no parent
	// yet can still be pointed at an arbitrary one.
	const isFork = $derived(!!parentWorkspaceId)
	// A dev workspace is a standing environment, torn down by detaching it in the
	// dev-workspace settings — never by an archive/delete button sitting next to the
	// merge it is here to perform.
	const isDevWorkspace = $derived(!!currentWorkspaceData?.is_dev_workspace)
	const hasCompareTarget = $derived(!!compareTargetId)

	// Mode is seeded from the URL (?mode=draft|fork). `draft` is valid for any
	// workspace, so it resolves immediately. `fork` is only valid for an actual
	// fork, so it (like an absent mode) defers to the effect below, which falls
	// back to draft for a non-fork once the workspace list has loaded (so `isFork`
	// is known) — otherwise `?mode=fork` on a non-fork would strand the page on the
	// fork UI, which can't render without a parent.
	const urlMode = page.url.searchParams.get('mode')
	let mode = $state<CompareMode>(urlMode === 'draft' ? 'draft' : 'fork')
	let modeResolved = $state(urlMode === 'draft')

	// Which fork direction to restore when switching back from draft mode. The
	// merged toggle (CompareModeToggle, rendered inside each card) reports its
	// selection here; the page only swaps which comparison component is shown.
	// `?dir=update` opens on the other one, for callers that already know which
	// direction has something in it (the fork banner's CTA, the "not in the dev
	// workspace yet" prompt).
	let forkDirection = $state<'deploy_to' | 'update'>(
		page.url.searchParams.get('dir') === 'update' ? 'update' : 'deploy_to'
	)

	// Explicit preselection via `?items=<kind:path,...>` (built by the chat's
	// open_page tool). Parsed synchronously from the live URL so it can never race
	// the children's select-all default. Present-but-empty means "preselect
	// nothing", distinct from absent (undefined → no mask).
	const urlItemsMask = $derived.by(() => {
		const v = page.url.searchParams.get(COMPARE_ITEMS_PARAM)
		return v === null ? undefined : parseItemsMaskParam(v)
	})

	// When reached via a session's Review button (`from_session=<chatId>`), preselect
	// only the items that chat modified. The mask is the chat's stored
	// `${UserDraftItemKind}:${storagePath}` set; undefined for a legacy chat (no
	// stored mask) → the children fall back to selecting all deployable items.
	// Derived from the live URL: an in-app navigation to this route with a
	// different from_session must reload the mask, not keep the first one.
	const fromChatId = $derived(page.url.searchParams.get('from_session'))
	let sessionMask = $state<Set<string> | undefined>(undefined)
	// The mask loads asynchronously, while the resolved value can legitimately be
	// undefined (legacy chat). The children must not run their select-all default
	// until the mask is known, else they'd race it and select everything. Ready
	// immediately when there's no session to read from.
	let sessionMaskReady = $state(!page.url.searchParams.get('from_session'))
	$effect(() => {
		const id = fromChatId
		sessionMask = undefined
		sessionMaskReady = !id
		if (!id) return
		untrack(() => {
			void readChatModifiedItems(id)
				.then((arr) => {
					// A slower read for a superseded chat id must not win.
					if (id !== untrack(() => fromChatId)) return
					sessionMask = arr ? new Set(arr) : undefined
				})
				.finally(() => {
					if (id === untrack(() => fromChatId)) sessionMaskReady = true
				})
		})
	})

	const chatMask = $derived(urlItemsMask ?? sessionMask)
	const chatMaskReady = $derived(urlItemsMask !== undefined || sessionMaskReady)

	function selectMode(v: 'deploy_to' | 'update' | 'draft') {
		if (v === 'draft') {
			mode = 'draft'
		} else {
			forkDirection = v
			mode = 'fork'
		}
	}

	// Draft count drives the "Deployed ↔ draft" toggle badge. Reads the shared
	// Workspace Drafts resource — count ≡ the draft list, and it refreshes itself
	// when a deploy/discard invalidates the workspace.
	const drafts = useWorkspaceDrafts(
		() => currentWorkspaceId,
		() => false,
		() => (isFork ? (parentWorkspaceId ?? undefined) : undefined)
	)
	// On a fork, match the badge to the default deploy-draft view, which hides
	// drafts unchanged from the parent (else a fresh fork shows a count over an
	// empty list).
	const draftCount = $derived(
		isFork ? drafts.items.filter((d) => d.unchanged_from_parent !== true).length : drafts.count
	)

	// Keys (`kind:path`) of fork items that are deployed *and* carry a pending
	// draft (has_draft, i.e. not draft_only). CompareWorkspaces uses this to flag
	// those rows — deploying/updating moves the deployed version, not the draft —
	// and to leave them out of the default selection. Raw apps map to the
	// `raw_app:` diff kind. draft_only items (never deployed) are excluded: they
	// don't appear in the fork comparison as deployed rows.
	const draftKeys = $derived(
		new Set(
			drafts.items
				.filter((d) => !d.draft_only)
				.map((d) => `${d.raw_app ? 'raw_app' : d.kind}:${d.path}`)
		)
	)

	// Per-direction counts for the merged toggle badges. Deployable = items ahead
	// (fork has changes the parent lacks); updateable = items behind, plus what the
	// parent has and the fork does not. Same predicate as the deploy list, so the
	// badge never counts rows the list won't show. Computed here so they show on the
	// toggle in draft mode too (where CompareDrafts has no comparison data of its
	// own). Typed helpers avoid a $state `never` inference quirk on `comparison`
	// inside $derived. A conflict (ahead AND behind) is intentionally counted in both
	// directions — it's actionable either way.
	function countDir(c: WorkspaceComparison | undefined, mergeIntoParent: boolean): number {
		return (
			c?.diffs.filter((d) => diffActionableInDirection(d, mergeIntoParent, isArbitraryTarget))
				.length ?? 0
		)
	}
	const deployCount = $derived(countDir(comparison, true))
	const updateCount = $derived(countDir(comparison, false))

	$effect(() => {
		if (modeResolved || !currentWorkspaceData) return
		if (!hasCompareTarget) {
			untrack(() => {
				// An explicit ?mode=fork with nothing to compare against is how the dev
				// workspace settings send a root workspace here to pick an arbitrary
				// target; the fork view renders that prompt. Anything else falls back to
				// drafts, the only view a workspace with no destination can fill.
				mode = urlMode === 'fork' ? 'fork' : 'draft'
				modeResolved = true
			})
			return
		}
		// An explicit ?mode=fork is only deferred (not latched at init) so the
		// non-fork fallback above can veto it — on a real fork, honor it as is.
		if (urlMode === 'fork') {
			untrack(() => {
				mode = 'fork'
				modeResolved = true
			})
			return
		}
		// A fork reached with a preselection mask but no ?mode= must land on the
		// view where the masked items actually are: a chat's pending drafts have no
		// fork-diff row, so fork mode would open with none of them selected. Defer
		// until the mask and the draft list are known, then prefer the draft view
		// when any masked item is a pending draft; else keep the fork comparison.
		if (!chatMaskReady) return
		const mask = chatMask
		if (mask?.size) {
			if (drafts.loading) return
			const masksDraft = drafts.items.some((d) => maskHasDraftRow(mask, d))
			untrack(() => {
				mode = masksDraft ? 'draft' : 'fork'
				modeResolved = true
			})
			return
		}
		untrack(() => {
			mode = 'fork'
			modeResolved = true
		})
	})

	// Several requests can be in flight at once — a retarget, a navigation, and the
	// post-deploy catch-up polls all issue their own. Only the most recently issued
	// may land: an older one carries the previous target's comparison, or an error
	// that has nothing to do with the pair now on screen.
	let comparisonReq = 0
	async function checkForChanges() {
		if (!currentWorkspaceId || !compareTargetId) {
			return
		}
		const seq = ++comparisonReq
		// The pair this request is about, read before the await: by the time it answers the page
		// may be on another one, and the answer belongs to the pair that was asked.
		const pair = pairKey
		comparisonLoading = true

		try {
			const result = await fetchWorkspaceComparison(compareTargetId, currentWorkspaceId)
			if (seq !== comparisonReq) return
			comparisonResult = { pair, value: result }
			comparisonError = undefined
		} catch (e: any) {
			if (seq !== comparisonReq) return
			comparisonError = e?.body ?? e?.message ?? String(e)
			console.error('Failed to compare workspaces:', e)
		} finally {
			// Only the request still being waited on clears the flag: an older one landing late
			// would otherwise report the newer pair as answered.
			if (seq === comparisonReq) comparisonLoading = false
		}
	}

	let comparisonError = $state<string | undefined>(undefined)
	/** Tracked rather than inferred from an absent comparison: a failed one is also absent, and a
	 *  spinner that never stops is worse than a message. */
	let comparisonLoading = $state(false)

	$effect(() => {
		;[currentWorkspaceId, compareTargetId]

		untrack(() => {
			comparisonError = undefined
			// The flag belongs to `checkForChanges`, which raises it per request and lowers it for
			// the one still being waited on. Raising it here too left it stuck on the pair that
			// function refuses — no workspace, or no target yet.
			checkForChanges()
		})
	})

	// Seeding the candidate set is what makes an arbitrary pair comparable at all;
	// the comparison that follows is the expensive part, since it evaluates every
	// candidate. Both are driven from here so the button reports the whole wait.
	/** The pair being scanned, so the button reports the wait for the pair it belongs to and not
	 *  for whichever one the trail moved to meanwhile. */
	let scanningPair = $state<string | undefined>(undefined)
	const scanning = $derived(scanningPair === pairKey)
	async function computeFullScan() {
		if (!currentWorkspaceId || !compareTargetId) return
		// Captured before the await, and used afterwards in place of the live values: the scan
		// seeded this pair, so this is the pair to invalidate, re-compare and name in the toast.
		const scanWorkspace = currentWorkspaceId
		const scanTarget = compareTargetId
		const pair = pairKey
		scanningPair = pair
		comparisonError = undefined
		try {
			const res = await WorkspaceService.seedFullDiffScan({
				workspace: scanWorkspace,
				targetWorkspaceId: scanTarget
			})
			invalidateWorkspaceComparison(scanTarget)
			// Only worth re-asking while the page is still on the pair that was scanned; it asks
			// for the pair it moved to on its own.
			if (pairKey === pair) await checkForChanges()
			// Each outcome says what actually happened. Off the scanned pair nothing was compared —
			// the re-ask above is skipped — so only the seed can be reported; `comparisonError`
			// there belongs to another pair and says nothing about this one. On it, the seed can
			// still have succeeded while the comparison reading it failed, and claiming "compared"
			// would be a lie with the seeded candidates sitting there for the retry the card offers.
			if (pairKey !== pair) {
				sendUserToast(`Seeded ${res.candidates} items for ${scanTarget}`)
			} else if (comparisonError) {
				sendUserToast(
					`Seeded ${res.candidates} items but the comparison failed: ${comparisonError}`,
					true
				)
			} else {
				sendUserToast(`Compared ${res.candidates} items with ${scanTarget}`)
			}
		} catch (e: any) {
			sendUserToast(`Failed to compute the diff: ${e?.body ?? e}`, true)
		} finally {
			// Only the scan still being waited on stops the button; a later one owns it now.
			if (scanningPair === pair) scanningPair = undefined
		}
	}

	function selectTarget(target: string) {
		const url = new URL(page.url)
		if (target === parentWorkspaceId) {
			url.searchParams.delete('target')
		} else {
			url.searchParams.set('target', target)
		}
		url.searchParams.set('mode', 'fork')
		// The mode is seeded from the URL at init only, so picking a target from the
		// draft view (or from the no-target prompt) has to switch the view itself.
		mode = 'fork'
		modeResolved = true
		goto(`${url.pathname}${url.search}`)
	}

	// Refresh the *fork comparison* after a child mutates state (deploy / update /
	// discard). The Draft Count refreshes itself (the mutation invalidates the
	// Workspace Drafts resource). The fork comparison (workspace_diff) is
	// recomputed *asynchronously* (~hundreds of ms after the action), so an
	// immediate re-fetch returns the pre-change diff — re-poll a few times to let
	// the tally catch up.
	let comparisonPollTimers: ReturnType<typeof setTimeout>[] = []
	function refreshCounts() {
		checkForChanges()
		comparisonPollTimers.forEach(clearTimeout)
		comparisonPollTimers = [800, 1800, 3500].map((delay) =>
			setTimeout(() => checkForChanges(), delay)
		)
	}

	// Don't let the catch-up timers fire after navigating away (network call +
	// $state write on a gone component).
	onDestroy(() => comparisonPollTimers.forEach(clearTimeout))

	// Fork lifecycle actions — placed in the page header so they're available
	// regardless of merge state. Both go through a confirmation modal because
	// archive is reversible-ish but delete is irreversible, and either way the
	// user is about to navigate away from this page.
	let archiveConfirmOpen = $state(false)
	let deleteConfirmOpen = $state(false)
	let acting = $state(false)

	async function afterForkGone() {
		// The workspace list was already refreshed by reconcileAfterWorkspaceChange
		// (so the just-removed fork is gone from it); land the user on the parent if
		// it's still accessible.
		if (parentWorkspaceId && $userWorkspaces.find((w) => w.id === parentWorkspaceId)) {
			switchWorkspace(parentWorkspaceId)
			await goto('/')
		} else {
			await goto('/user/workspaces')
		}
	}

	async function confirmArchive() {
		archiveConfirmOpen = false
		if (!currentWorkspaceId) return
		acting = true
		try {
			await WorkspaceService.archiveWorkspace({ workspace: currentWorkspaceId })
			sendUserToast(`Archived fork ${currentWorkspaceId}`)
			// Client session cleanup is best-effort: a local IndexedDB failure must
			// not falsely report the (already successful) archive as failed, nor
			// block navigation away from the now-archived fork.
			try {
				await archiveSessionsForWorkspace(currentWorkspaceId)
				await reconcileAfterWorkspaceChange()
			} catch (e) {
				console.error('Session cleanup after fork archive failed', e)
			}
			await afterForkGone()
		} catch (e: any) {
			sendUserToast(`Failed to archive fork: ${e?.body ?? e}`, true)
		} finally {
			acting = false
		}
	}

	async function confirmDelete() {
		deleteConfirmOpen = false
		if (!currentWorkspaceId) return
		acting = true
		try {
			await WorkspaceService.deleteWorkspace({ workspace: currentWorkspaceId })
			sendUserToast(`Deleted fork ${currentWorkspaceId}`)
			// Client session cleanup is best-effort: a local IndexedDB failure must
			// not abort the redirect after a successful delete, leaving the user on
			// the now-deleted workspace path.
			try {
				await deleteSessionsForWorkspace(currentWorkspaceId)
				await reconcileAfterWorkspaceChange()
			} catch (e) {
				console.error('Session cleanup after fork delete failed', e)
			}
			await afterForkGone()
		} catch (e: any) {
			sendUserToast(`Failed to delete fork: ${e?.body ?? e}`, true)
		} finally {
			acting = false
		}
	}
</script>

<!-- Named here rather than left to the route: the breadcrumb's fallback reads the first segment,
     which would call this page "Forks" — the thing it compares, not what it is.
     `actingWorkspaceId` only when the page is comparing a workspace other than the one the app is
     pointed at, which is how a session's Review button and the prod→dev link open it: the trail
     would otherwise name the parent while the page compares the fork, and its home link would
     lead somewhere the page is not. -->
<PageHeaderContent
	section={{ label: 'Compare' }}
	actingWorkspaceId={currentWorkspaceId !== $workspaceStore ? currentWorkspaceId : undefined}
/>

<CenteredPage>
	<PageHeader title="Compare & Deploy">
		<div class="flex flex-row gap-2 items-center">
			<!-- The merged compare toggle (fork direction + deployed↔draft) lives inside
			     each comparison card; only the fork lifecycle actions remain in the page
			     header, and only for a throwaway fork. -->
			{#if isFork && !isDevWorkspace}
				<Button
					variant="default"
					color="light"
					size="xs"
					startIcon={{ icon: Archive }}
					disabled={acting}
					on:click={() => (archiveConfirmOpen = true)}
				>
					Archive fork
				</Button>
				<Button
					variant="default"
					color="red"
					size="xs"
					startIcon={{ icon: Trash2 }}
					disabled={acting}
					on:click={() => (deleteConfirmOpen = true)}
				>
					Delete fork
				</Button>
			{/if}
		</div>
	</PageHeader>
	{#if !currentWorkspaceId}
		No workspace selected
	{:else if mode === 'draft'}
		<CompareDrafts
			{currentWorkspaceId}
			draftItems={drafts.items}
			draftsLoading={drafts.loading}
			onChanged={refreshCounts}
			{isFork}
			parentWorkspaceId={parentWorkspaceId ?? undefined}
			{compareTargetId}
			oneWayCompare={isArbitraryTarget}
			{deployCount}
			{updateCount}
			{draftCount}
			{chatMask}
			{chatMaskReady}
			onModeSelected={selectMode}
		/>
	{:else if compareTargetId}
		<!-- Remount on either side of the pair changing: the merge card owns a selection, a deploy
		     direction, per-item deployment statuses, item summaries, pinned rows, deploy
		     permissions and CI results — none of which carry over to a different pair. Keyed on
		     the target alone, two forks of one parent shared a mount, since the target is the
		     parent they have in common. -->
		{#key pairKey}
			<CompareWorkspaces
				{currentWorkspaceId}
				parentWorkspaceId={compareTargetId}
				lineageParentId={parentWorkspaceId ?? undefined}
				{isArbitraryTarget}
				fullScanAt={comparison?.full_scan_at}
				{scanning}
				onScan={computeFullScan}
				onRetry={checkForChanges}
				onSelectTarget={selectTarget}
				{comparison}
				{comparisonError}
				{comparisonLoading}
				initialMergeIntoParent={forkDirection === 'deploy_to'}
				{deployCount}
				{updateCount}
				{draftCount}
				{draftKeys}
				{chatMask}
				{chatMaskReady}
				maskAppliesToUpdate={urlItemsMask !== undefined}
				onChanged={refreshCounts}
				onModeSelected={selectMode}
			/>
		{/key}
	{:else}
		<div class="flex flex-col gap-3 items-start border rounded-md bg-surface p-4 mt-2">
			<p class="text-sm text-secondary max-w-2xl">
				Workspace <span class="font-mono text-primary">{currentWorkspaceId}</span> has no parent workspace
				to merge into. Pick any workspace you administer to compare against it instead — this is meant
				for one-off migrations, and computes a full diff over both workspaces.
			</p>
			<CompareTargetPicker {currentWorkspaceId} targetWorkspaceId="" onSelected={selectTarget} />
		</div>
	{/if}
</CenteredPage>

<ConfirmationModal
	open={archiveConfirmOpen}
	title="Archive fork"
	confirmationText="Archive"
	onConfirmed={confirmArchive}
	onCanceled={() => (archiveConfirmOpen = false)}
>
	<p>
		Archive forked workspace <span class="font-mono font-medium text-primary"
			>{currentWorkspaceId}</span
		>? It will be hidden from the workspace picker; a superadmin can restore it from instance
		settings later. Its content is kept and its workspace id stays reserved — use Delete fork
		instead if you want to reuse the id for a new fork.
	</p>
</ConfirmationModal>

<ConfirmationModal
	open={deleteConfirmOpen}
	title="Delete fork"
	confirmationText="Delete"
	onConfirmed={confirmDelete}
	onCanceled={() => (deleteConfirmOpen = false)}
>
	<p>
		Permanently delete forked workspace <span class="font-mono font-medium text-primary"
			>{currentWorkspaceId}</span
		>? This cannot be undone. Any sessions still bound to this fork will show as "Fork — no longer
		available" in the sidebar.
	</p>
</ConfirmationModal>
