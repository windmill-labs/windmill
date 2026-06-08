<script lang="ts">
	import CompareWorkspaces from '$lib/components/CompareWorkspaces.svelte'
	import { WorkspaceService, type WorkspaceComparison } from '$lib/gen'
	import { page } from '$app/state'
	import { userWorkspaces, usersWorkspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { Archive, Trash2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'

	let comparison: WorkspaceComparison | undefined = $state(undefined)

	let currentWorkspaceId: string | undefined = $state(
		page.url.searchParams.get('workspace_id') ?? undefined
	)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === currentWorkspaceId))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)

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

	const isFork = $derived(!!parentWorkspaceId && currentWorkspaceId?.startsWith('wm-fork-'))
</script>

<CenteredPage>
	<PageHeader title="Merge workspaces">
		{#if isFork}
			<div class="flex flex-row gap-2 items-center">
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
			</div>
		{/if}
	</PageHeader>
	{#if currentWorkspaceId && parentWorkspaceId}
		<CompareWorkspaces {currentWorkspaceId} {parentWorkspaceId} {comparison} />
	{/if}
	{#if !currentWorkspaceId}
		No workspace selected
	{:else if !parentWorkspaceId}
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
