<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Badge from './common/badge/Badge.svelte'
	import { Loader2, GitFork, ArrowRight, ArrowLeft, Check, X, ChevronDown, ChevronRight } from 'lucide-svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	// Type definitions - these will be auto-generated from OpenAPI spec
	interface WorkspaceDiffItem {
		path: string
		name: string
		item_type: string
		status: 'added' | 'deleted' | 'modified' | 'unchanged'
		versions_ahead: number
		versions_behind: number
		changes: Array<{
			change_type: string
			has_changes: boolean
		}>
	}

	interface WorkspaceComparisonResult {
		fork_workspace_id: string
		parent_workspace_id: string
		items: WorkspaceDiffItem[]
		summary: {
			total_changes: number
			added: number
			deleted: number
			modified: number
			unchanged: number
		}
	}

	export let workspaceId: string

	let diffDrawer: Drawer
	let loading = false
	let comparisonResult: WorkspaceComparisonResult | undefined
	let deployDirection: 'deploy' | 'update' = 'deploy'
	let selectedItems = new Set<string>()
	let expandedItems = new Set<string>()
	let showUnchanged = false

	$: isForkedWorkspace = workspaceId?.startsWith('wm-fork-')
	$: if (isForkedWorkspace && workspaceId) {
		checkForChanges()
	}

	async function checkForChanges() {
		loading = true
		try {
			// TODO: Replace with WorkspaceService.workspaceCompareWithParent once API is regenerated
			const response = await fetch(`/api/w/${workspaceId}/workspaces/compare_with_parent`, {
				headers: {
					'Content-Type': 'application/json'
				}
			})
			if (response.ok) {
				comparisonResult = await response.json()
			}
		} catch (err) {
			console.error('Failed to compare workspaces:', err)
		} finally {
			loading = false
		}
	}

	function openDiffViewer() {
		diffDrawer?.openDrawer()
	}

	function getItemKey(item: WorkspaceDiffItem): string {
		return `${item.item_type}:${item.path}`
	}

	function toggleItemSelection(item: WorkspaceDiffItem) {
		const key = getItemKey(item)
		if (selectedItems.has(key)) {
			selectedItems.delete(key)
		} else {
			selectedItems.add(key)
		}
		selectedItems = selectedItems
	}

	function toggleItemExpansion(item: WorkspaceDiffItem) {
		const key = getItemKey(item)
		if (expandedItems.has(key)) {
			expandedItems.delete(key)
		} else {
			expandedItems.add(key)
		}
		expandedItems = expandedItems
	}

	function selectAll() {
		const filteredItems = getFilteredItems()
		filteredItems.forEach(item => {
			selectedItems.add(getItemKey(item))
		})
		selectedItems = selectedItems
	}

	function deselectAll() {
		selectedItems.clear()
		selectedItems = selectedItems
	}

	function getFilteredItems(): WorkspaceDiffItem[] {
		if (!comparisonResult) return []
		
		return comparisonResult.items.filter(item => {
			// Filter unchanged items if needed
			if (!showUnchanged && item.status === 'unchanged') {
				return false
			}

			// Filter based on deploy direction
			if (deployDirection === 'deploy') {
				// Deploy to parent: only show items that are ahead (added or modified in fork)
				return item.status === 'added' || (item.status === 'modified' && item.versions_ahead > 0)
			} else {
				// Update from parent: only show items that are behind (deleted or modified in parent)
				return item.status === 'deleted' || (item.status === 'modified' && item.versions_behind > 0)
			}
		})
	}

	async function deployChanges() {
		if (selectedItems.size === 0) {
			sendUserToast('Please select items to deploy', true)
			return
		}

		loading = true
		try {
			const items = Array.from(selectedItems).map(key => {
				const [item_type, path] = key.split(':')
				return { item_type, path }
			})

			const response = await fetch(`/api/w/${workspaceId}/workspaces/deploy_to_parent`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					items,
					direction: deployDirection
				})
			})

			if (response.ok) {
				const result = await response.text()
				sendUserToast(result, false)
				// Refresh comparison after deployment
				await checkForChanges()
				selectedItems.clear()
				selectedItems = selectedItems
			} else {
				const error = await response.text()
				sendUserToast(`Deployment failed: ${error}`, true)
			}
		} catch (err) {
			console.error('Failed to deploy changes:', err)
			sendUserToast('Failed to deploy changes', true)
		} finally {
			loading = false
		}
	}

	function getStatusBadgeColor(status: string) {
		switch (status) {
			case 'added': return 'green'
			case 'deleted': return 'red'
			case 'modified': return 'blue'
			case 'unchanged': return 'gray'
			default: return 'gray'
		}
	}

	function getItemTypeIcon(type: string) {
		switch (type) {
			case 'script': return '📜'
			case 'flow': return '🔀'
			case 'app': return '📱'
			case 'resource': return '🔧'
			case 'variable': return '📦'
			default: return '📄'
		}
	}
</script>

{#if isForkedWorkspace}
	{#if loading}
		<div class="flex items-center gap-2 p-2 bg-blue-50 dark:bg-blue-900/20 rounded">
			<Loader2 class="animate-spin" size={16} />
			<span class="text-sm">Checking for changes...</span>
		</div>
	{:else if comparisonResult && comparisonResult.summary.total_changes > 0}
		<Alert type="info" title="Forked Workspace" class="mb-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<GitFork size={16} />
					<span>
						This is a forked workspace with {comparisonResult.summary.total_changes} unmerged changes
					</span>
				</div>
				<Button size="sm" on:click={openDiffViewer}>
					View Changes
				</Button>
			</div>
		</Alert>
	{/if}

	<Drawer bind:this={diffDrawer} size="1200px">
		<DrawerContent title="Workspace Fork Differences" on:close={diffDrawer.closeDrawer}>
			{#if comparisonResult}
				<div class="flex flex-col gap-4 h-full">
					<!-- Header Controls -->
					<div class="flex items-center justify-between border-b pb-4">
						<div class="flex items-center gap-4">
							<div class="flex items-center gap-2">
								<span class="text-sm font-medium">Direction:</span>
								<select
									bind:value={deployDirection}
									class="px-2 py-1 rounded border"
								>
									<option value="deploy">Deploy to Parent</option>
									<option value="update">Update from Parent</option>
								</select>
							</div>
							<div class="flex items-center gap-2">
								<input
									type="checkbox"
									bind:checked={showUnchanged}
									id="show-unchanged"
								/>
								<label for="show-unchanged" class="text-sm">Show unchanged items</label>
							</div>
						</div>
						<div class="flex items-center gap-2">
							<Button size="xs" variant="border" on:click={selectAll}>
								Select All
							</Button>
							<Button size="xs" variant="border" on:click={deselectAll}>
								Deselect All
							</Button>
						</div>
					</div>

					<!-- Summary Stats -->
					<div class="grid grid-cols-4 gap-4">
						<div class="bg-green-50 dark:bg-green-900/20 p-3 rounded">
							<div class="text-2xl font-bold text-green-600 dark:text-green-400">
								{comparisonResult.summary.added}
							</div>
							<div class="text-sm text-gray-600 dark:text-gray-400">Added</div>
						</div>
						<div class="bg-blue-50 dark:bg-blue-900/20 p-3 rounded">
							<div class="text-2xl font-bold text-blue-600 dark:text-blue-400">
								{comparisonResult.summary.modified}
							</div>
							<div class="text-sm text-gray-600 dark:text-gray-400">Modified</div>
						</div>
						<div class="bg-red-50 dark:bg-red-900/20 p-3 rounded">
							<div class="text-2xl font-bold text-red-600 dark:text-red-400">
								{comparisonResult.summary.deleted}
							</div>
							<div class="text-sm text-gray-600 dark:text-gray-400">Deleted</div>
						</div>
						<div class="bg-gray-50 dark:bg-gray-900/20 p-3 rounded">
							<div class="text-2xl font-bold text-gray-600 dark:text-gray-400">
								{comparisonResult.summary.unchanged}
							</div>
							<div class="text-sm text-gray-600 dark:text-gray-400">Unchanged</div>
						</div>
					</div>

					<!-- Items List -->
					<div class="flex-1 overflow-y-auto border rounded">
						{#each getFilteredItems() as item}
							{@const isExpanded = expandedItems.has(getItemKey(item))}
							{@const isSelected = selectedItems.has(getItemKey(item))}
							{@const hasConflict = item.versions_ahead > 0 && item.versions_behind > 0}
							
							<div class="border-b">
								<div class="flex items-center gap-3 p-3 hover:bg-gray-50 dark:hover:bg-gray-800">
									<input
										type="checkbox"
										checked={isSelected}
										on:change={() => toggleItemSelection(item)}
										class="flex-shrink-0"
									/>
									
									<button
										on:click={() => toggleItemExpansion(item)}
										class="flex-shrink-0"
										disabled={item.status === 'unchanged'}
									>
										{#if item.status !== 'unchanged'}
											{#if isExpanded}
												<ChevronDown size={16} />
											{:else}
												<ChevronRight size={16} />
											{/if}
										{:else}
											<div class="w-4" />
										{/if}
									</button>

									<span class="text-lg flex-shrink-0">
										{getItemTypeIcon(item.item_type)}
									</span>

									<div class="flex-1 min-w-0">
										<div class="flex items-center gap-2">
											<span class="font-mono text-sm truncate">
												{item.path}
											</span>
											<Badge color={getStatusBadgeColor(item.status)}>
												{item.status}
											</Badge>
											{#if hasConflict}
												<Badge color="orange">Conflict</Badge>
											{/if}
										</div>
										{#if item.versions_ahead > 0 || item.versions_behind > 0}
											<div class="text-xs text-gray-600 dark:text-gray-400 mt-1">
												{#if item.versions_ahead > 0}
													<span class="text-green-600 dark:text-green-400">
														↑ {item.versions_ahead} ahead
													</span>
												{/if}
												{#if item.versions_behind > 0}
													<span class="text-red-600 dark:text-red-400 ml-2">
														↓ {item.versions_behind} behind
													</span>
												{/if}
											</div>
										{/if}
									</div>

									<div class="flex gap-1 flex-shrink-0">
										{#each item.changes as change}
											{#if change.has_changes}
												<Badge size="xs" color="blue">
													{change.change_type}
												</Badge>
											{/if}
										{/each}
									</div>
								</div>

								{#if isExpanded && item.status !== 'unchanged'}
									<div class="bg-gray-50 dark:bg-gray-900 p-4 border-t">
										<div class="text-sm text-gray-600 dark:text-gray-400">
											{#if item.changes.length > 0}
												<div class="mb-2">Changes detected in:</div>
												<ul class="list-disc list-inside">
													{#each item.changes as change}
														{#if change.has_changes}
															<li>{change.change_type}</li>
														{/if}
													{/each}
												</ul>
											{/if}
											{#if hasConflict}
												<Alert type="warning" title="Conflict" class="mt-2">
													This item has been modified in both the fork and parent workspace.
													Deploying will overwrite changes.
												</Alert>
											{/if}
										</div>
										<!-- TODO: Add line-by-line diff viewer here for scripts/flows -->
									</div>
								{/if}
							</div>
						{/each}

						{#if getFilteredItems().length === 0}
							<div class="p-8 text-center text-gray-500">
								No items to {deployDirection === 'deploy' ? 'deploy' : 'update'}
							</div>
						{/if}
					</div>
				</div>
			{/if}

			{#snippet actions()}
				<Button
					disabled={selectedItems.size === 0}
					on:click={deployChanges}
					startIcon={{ icon: deployDirection === 'deploy' ? ArrowRight : ArrowLeft }}
				>
					{deployDirection === 'deploy' ? 'Deploy to Parent' : 'Update from Parent'}
					{#if selectedItems.size > 0}
						({selectedItems.size} items)
					{/if}
				</Button>
			{/snippet}
		</DrawerContent>
	</Drawer>
{/if}