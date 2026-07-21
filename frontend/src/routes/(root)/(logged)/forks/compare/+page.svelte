<script lang="ts">
	import CompareWorkspaces from '$lib/components/CompareWorkspaces.svelte'
	import CompareDrafts from '$lib/components/CompareDrafts.svelte'
	import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'
	import {
		archiveSessionsForWorkspace,
		deleteSessionsForWorkspace,
		reconcileAfterWorkspaceChange
	} from '$lib/components/sessions/sessionState.svelte'
	import { useWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
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

	let comparison: WorkspaceComparison | undefined = $state(undefined)

	let currentWorkspaceId: string | undefined = $state(
		page.url.searchParams.get('workspace_id') ?? $workspaceStore ?? undefined
	)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === currentWorkspaceId))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)
	// Fork/dev workspaces are identified by their parent link, not the `wm-fork-` id prefix.
	const isFork = $derived(!!parentWorkspaceId)

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
	let forkDirection = $state<'deploy_to' | 'update'>('deploy_to')

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
	// (fork has changes the parent lacks); updateable = items behind. Computed
	// here so they show on the toggle in draft mode too (where CompareDrafts has
	// no comparison data of its own). Typed helpers avoid a $state `never`
	// inference quirk on `comparison` inside $derived. A conflict (ahead AND
	// behind) is intentionally counted in both directions — it's actionable either
	// way.
	function countDir(c: WorkspaceComparison | undefined, dir: 'ahead' | 'behind'): number {
		return c?.diffs.filter((d) => d[dir] > 0).length ?? 0
	}
	const deployCount = $derived(countDir(comparison, 'ahead'))
	const updateCount = $derived(countDir(comparison, 'behind'))

	$effect(() => {
		if (modeResolved || !currentWorkspaceData) return
		if (!isFork) {
			untrack(() => {
				mode = 'draft'
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

	async function checkForChanges() {
		if (!currentWorkspaceId || !parentWorkspaceId) {
			return
		}

		try {
			const result = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: currentWorkspaceId
			})

			comparison = result
		} catch (e) {
			console.error('Failed to compare workspaces:', e)
		}
	}

	$effect(() => {
		;[currentWorkspaceId, parentWorkspaceId]

		untrack(() => checkForChanges())
	})

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

<CenteredPage>
	<PageHeader title="Compare & Deploy">
		<div class="flex flex-row gap-2 items-center">
			<!-- The merged compare toggle (fork direction + deployed↔draft) now lives
			     inside each comparison card; only the fork lifecycle actions remain
			     in the page header. -->
			{#if isFork}
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
			{deployCount}
			{updateCount}
			{draftCount}
			{chatMask}
			{chatMaskReady}
			onModeSelected={selectMode}
		/>
	{:else if parentWorkspaceId}
		<CompareWorkspaces
			{currentWorkspaceId}
			{parentWorkspaceId}
			{comparison}
			initialMergeIntoParent={forkDirection === 'deploy_to'}
			{deployCount}
			{updateCount}
			{draftCount}
			{draftKeys}
			{chatMask}
			{chatMaskReady}
			onChanged={refreshCounts}
			onModeSelected={selectMode}
		/>
	{:else}
		workspace {currentWorkspaceId} has no parent workspace
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
