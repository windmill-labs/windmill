<script lang="ts">
	import CompareWorkspaces from '$lib/components/CompareWorkspaces.svelte'
	import CompareDrafts from '$lib/components/CompareDrafts.svelte'
	import { DraftService, WorkspaceService, type WorkspaceComparison } from '$lib/gen'
	import { page } from '$app/state'
	import { userWorkspaces, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Archive, Trash2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'

	type CompareMode = 'fork' | 'draft'

	let comparison: WorkspaceComparison | undefined = $state(undefined)

	let currentWorkspaceId: string | undefined = $state(
		page.url.searchParams.get('workspace_id') ?? $workspaceStore ?? undefined
	)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === currentWorkspaceId))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)
	const isFork = $derived(!!parentWorkspaceId && currentWorkspaceId?.startsWith('wm-fork-'))

	// Mode is seeded from the URL (?mode=draft|fork). When absent it defaults to
	// fork in a fork workspace, draft otherwise — resolved once the workspace list
	// has loaded (so `isFork` is known).
	const urlMode = page.url.searchParams.get('mode')
	let mode = $state<CompareMode>(urlMode === 'draft' || urlMode === 'fork' ? urlMode : 'fork')
	let modeResolved = $state(urlMode === 'draft' || urlMode === 'fork')

	// Which fork direction to restore when switching back from draft mode. The
	// merged toggle (CompareModeToggle, rendered inside each card) reports its
	// selection here; the page only swaps which comparison component is shown.
	let forkDirection = $state<'deploy_to' | 'update'>('deploy_to')

	function selectMode(v: 'deploy_to' | 'update' | 'draft') {
		if (v === 'draft') {
			mode = 'draft'
		} else {
			forkDirection = v
			mode = 'fork'
		}
	}

	// Draft count drives the "Deployed ↔ draft" toggle badge. Seeded from the
	// count endpoint on load; kept live by CompareDrafts via onCountChange.
	let draftCount = $state(0)

	async function fetchDraftCount() {
		if (!currentWorkspaceId) return
		try {
			draftCount = (await DraftService.countDrafts({ workspace: currentWorkspaceId })).count
		} catch (e) {
			console.error('Failed to count drafts:', e)
		}
	}

	$effect(() => {
		;[currentWorkspaceId]
		untrack(() => fetchDraftCount())
	})

	$effect(() => {
		if (modeResolved || !currentWorkspaceData) return
		untrack(() => {
			mode = isFork ? 'fork' : 'draft'
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

	// Fork lifecycle actions — placed in the page header so they're available
	// regardless of merge state. Both go through a confirmation modal because
	// archive is reversible-ish but delete is irreversible, and either way the
	// user is about to navigate away from this page.
	let archiveConfirmOpen = $state(false)
	let deleteConfirmOpen = $state(false)
	let acting = $state(false)

	async function afterForkGone() {
		// Mirror SidebarContent.deleteFork (B1): refresh the workspace list
		// rather than letting `clearStores()` null it, then land the user on
		// the parent if still accessible.
		try {
			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		} catch (e) {
			console.error('Failed to refresh workspaces', e)
		}
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
			await afterForkGone()
		} catch (e: any) {
			sendUserToast(`Failed to delete fork: ${e?.body ?? e}`, true)
		} finally {
			acting = false
		}
	}
</script>

<CenteredPage>
	<PageHeader title="Compare workspaces">
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
			onCountChange={(n) => (draftCount = n)}
			{isFork}
			parentWorkspaceId={parentWorkspaceId ?? undefined}
			{draftCount}
			onModeSelected={selectMode}
		/>
	{:else if parentWorkspaceId}
		<CompareWorkspaces
			{currentWorkspaceId}
			{parentWorkspaceId}
			{comparison}
			initialMergeIntoParent={forkDirection === 'deploy_to'}
			{draftCount}
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
		settings later.
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
