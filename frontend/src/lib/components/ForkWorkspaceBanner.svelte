<script lang="ts">
	import { workspaceStore, userWorkspaces } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import type { WorkspaceComparison } from '$lib/gen'
	import { Button } from './common'
	import { Alert } from './common'
	import { InfoIcon, GitBranch, ArrowUpRight, ArrowDownRight, AlertTriangle } from 'lucide-svelte'
	import WorkspaceComparisonDrawer from './WorkspaceComparisonDrawer.svelte'

	let comparisonDrawer: WorkspaceComparisonDrawer | undefined = undefined
	let loading = false
	let comparison: WorkspaceComparison | undefined = undefined
	let error: string | undefined = undefined
	let isVisible = false

	$: currentWorkspace = $workspaceStore
	$: currentWorkspaceData = $userWorkspaces.find(w => w.id === currentWorkspace)
	$: isFork = currentWorkspace?.startsWith('wm-fork-') ?? false
	$: parentWorkspaceId = currentWorkspaceData?.parent_workspace_id
	
	// Determine if we should show the banner
	$: if (isFork && currentWorkspace) {
		checkForChanges()
	} else {
		isVisible = false
		comparison = undefined
	}

	async function checkForChanges() {
		if (!currentWorkspace || !parentWorkspaceId) {
			return
		}

		loading = true
		error = undefined
		
		try {
			// Compare with parent workspace
			const result = await WorkspaceService.compareWorkspaces({
				workspace: currentWorkspace,
				targetWorkspaceId: parentWorkspaceId
			})
			
			comparison = result
			isVisible = result.summary.total_diffs > 0
		} catch (e) {
			console.error('Failed to compare workspaces:', e)
			error = 'Failed to check for changes'
			// Still show banner if there's an error, but with error message
			isVisible = true
		} finally {
			loading = false
		}
	}

	function openComparisonDrawer() {
		comparisonDrawer?.open()
	}
</script>

{#if isVisible && isFork}
	<div class="w-full bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800">
		<div class="px-4 py-2">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<GitBranch class="w-4 h-4 text-blue-600 dark:text-blue-400" />
					<div class="text-sm">
						<span class="font-medium text-blue-900 dark:text-blue-100">
							Fork Workspace
						</span>
						{#if parentWorkspaceId}
							<span class="text-blue-700 dark:text-blue-300 ml-1">
								(parent: {parentWorkspaceId})
							</span>
						{/if}
					</div>
					
					{#if loading}
						<span class="text-xs text-blue-600 dark:text-blue-400">
							Checking for changes...
						</span>
					{:else if error}
						<span class="text-xs text-red-600 dark:text-red-400">
							{error}
						</span>
					{:else if comparison}
						<div class="flex items-center gap-4 text-xs">
							{#if comparison.summary.total_diffs > 0}
								<div class="flex items-center gap-2">
									{#if comparison.summary.scripts_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.scripts_changed} script{comparison.summary.scripts_changed !== 1 ? 's' : ''}
										</span>
									{/if}
									{#if comparison.summary.flows_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.flows_changed} flow{comparison.summary.flows_changed !== 1 ? 's' : ''}
										</span>
									{/if}
									{#if comparison.summary.apps_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.apps_changed} app{comparison.summary.apps_changed !== 1 ? 's' : ''}
										</span>
									{/if}
									{#if comparison.summary.resources_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.resources_changed} resource{comparison.summary.resources_changed !== 1 ? 's' : ''}
										</span>
									{/if}
									{#if comparison.summary.variables_changed > 0}
										<span class="text-blue-700 dark:text-blue-300">
											{comparison.summary.variables_changed} variable{comparison.summary.variables_changed !== 1 ? 's' : ''}
										</span>
									{/if}
								</div>
								
								{#if comparison.summary.conflicts > 0}
									<div class="flex items-center gap-1 text-orange-600 dark:text-orange-400">
										<AlertTriangle class="w-3 h-3" />
										<span>{comparison.summary.conflicts} conflict{comparison.summary.conflicts !== 1 ? 's' : ''}</span>
									</div>
								{/if}
							{:else}
								<span class="text-blue-600 dark:text-blue-400">
									No changes to deploy
								</span>
							{/if}
						</div>
					{/if}
				</div>
				
				<div class="flex items-center gap-2">
					{#if comparison && comparison.summary.total_diffs > 0}
						<Button
							size="xs"
							color="blue"
							on:click={openComparisonDrawer}
						>
							Review & Deploy Changes
						</Button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<WorkspaceComparisonDrawer
	bind:this={comparisonDrawer}
	{comparison}
	sourceWorkspace={currentWorkspace}
	targetWorkspace={parentWorkspaceId ?? undefined}
	on:deployed={() => checkForChanges()}
/>