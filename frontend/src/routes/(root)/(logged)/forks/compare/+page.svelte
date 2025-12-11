<script lang="ts">
	import CompareWorkspaces from "$lib/components/CompareWorkspaces.svelte"
	import { WorkspaceService, type WorkspaceComparison } from "$lib/gen"
	import { page } from '$app/state'
	import { userWorkspaces } from "$lib/stores"
	import { untrack } from "svelte"
	import CenteredPage from "$lib/components/CenteredPage.svelte"
	import PageHeader from "$lib/components/PageHeader.svelte"

	let comparison: WorkspaceComparison | undefined = $state(undefined)

	let currentWorkspaceId: string | undefined = $state(page.url.searchParams.get('workspace_id') ?? undefined)

	let currentWorkspaceData = $derived($userWorkspaces.find((w) => w.id === currentWorkspaceId))
	let parentWorkspaceId = $derived(currentWorkspaceData?.parent_workspace_id)

	async function checkForChanges() { if (!currentWorkspaceId || !parentWorkspaceId) {
			return
		}

		// loading = true
		// error = undefined

		try {
			// Compare with parent workspace
			const result = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: currentWorkspaceId
			})

			comparison = result
			// isVisible = result.summary.total_diffs > 0
		} catch (e) {
			console.error('Failed to compare workspaces:', e)
			// error = 'Failed to check for changes'
			// Still show banner if there's an error, but with error message
			// isVisible = true
		} finally {
			// loading = false
		}
	}

	$effect(() => {
		[
			currentWorkspaceId,
			parentWorkspaceId,
		];

		untrack(() => checkForChanges())
	})



</script>
<CenteredPage>

<PageHeader
	title="Merge workspaces"
/>
	{#if currentWorkspaceId && parentWorkspaceId}
		<!-- <WorkspaceComparisonDrawer -->
		<!-- 	{comparison} -->
		<!-- 	sourceWorkspace={currentWorkspaceId} -->
		<!-- 	targetWorkspace={parentWorkspaceId} -->
		<!-- 	on:deployed={() => { -->
		<!-- 		sendUserToast('Changes deployed successfully') -->
		<!-- 	}} -->
		<!-- /> -->
		<CompareWorkspaces {currentWorkspaceId} {parentWorkspaceId} {comparison}/>
	{/if}
	{#if !currentWorkspaceId}
		No workspace selected
	{:else if !parentWorkspaceId}
		workspace {currentWorkspaceId} has no parent workspace
	{/if}
</CenteredPage>
