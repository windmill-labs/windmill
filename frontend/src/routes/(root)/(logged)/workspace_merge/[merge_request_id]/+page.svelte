<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/stores'
	import { onMount } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import { GitPullRequest, ArrowLeft, GitMerge, AlertTriangle, Check, X, Clock } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import DiffEditor from '$lib/components/DiffEditor.svelte'

	const mergeRequestId = $page.params.merge_request_id

	let mergeRequest = $state<any>(null)
	let changes = $state<any[]>([])
	let loading = $state(true)
	let merging = $state(false)
	let rejecting = $state(false)
	let selectedChange = $state<any>(null)
	let showDiff = $state(false)

	// Conflict resolution
	let resolvingConflict = $state(false)
	let conflictResolution = $state('')

	async function loadMergeRequest() {
		if (!$workspaceStore || !$userStore?.is_admin) {
			goto(`${base}/`)
			return
		}

		loading = true
		try {
			// Load merge request details
			const requestResponse = await fetch(`/api/w/${$workspaceStore}/workspaces/merge/merge_request/${mergeRequestId}`)
			if (requestResponse.ok) {
				mergeRequest = await requestResponse.json()
			} else {
				sendUserToast('Merge request not found', true)
				goto(`${base}/workspace_merge`)
				return
			}

			// Load changes
			const changesResponse = await fetch(`/api/w/${$workspaceStore}/workspaces/merge/merge_request/${mergeRequestId}/changes`)
			if (changesResponse.ok) {
				changes = await changesResponse.json()
			}

		} catch (error) {
			sendUserToast('Failed to load merge request', true)
			console.error(error)
		} finally {
			loading = false
		}
	}

	async function executeMerge() {
		if (!mergeRequest || merging) return

		merging = true
		try {
			const response = await fetch(`/api/w/${$workspaceStore}/workspaces/merge/merge_request/${mergeRequestId}/execute`, {
				method: 'POST'
			})

			if (response.ok) {
				sendUserToast('Merge completed successfully!')
				goto(`${base}/`)
			} else {
				const error = await response.text()
				sendUserToast(`Failed to merge: ${error}`, true)
			}
		} catch (error) {
			sendUserToast('Failed to execute merge', true)
			console.error(error)
		} finally {
			merging = false
		}
	}

	async function resolveConflict(changeId: number, strategy: string) {
		if (resolvingConflict) return

		resolvingConflict = true
		try {
			const response = await fetch(`/api/w/${$workspaceStore}/workspaces/merge/merge_request/${mergeRequestId}/resolve_conflict`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					change_id: changeId,
					resolution_strategy: strategy
				})
			})

			if (response.ok) {
				sendUserToast('Conflict resolved')
				await loadMergeRequest()
			} else {
				const error = await response.text()
				sendUserToast(`Failed to resolve conflict: ${error}`, true)
			}
		} catch (error) {
			sendUserToast('Failed to resolve conflict', true)
			console.error(error)
		} finally {
			resolvingConflict = false
		}
	}

	async function loadResourceContent(change: any) {
		selectedChange = change
		// Here we would load the actual content for diff viewing
		// For now, we'll show a placeholder
		showDiff = true
	}

	onMount(loadMergeRequest)

	// Computed values
	let conflictedChanges = $derived(changes.filter(c => c.has_conflict && !c.resolved))
	let resolvedChanges = $derived(changes.filter(c => c.resolved))
	let normalChanges = $derived(changes.filter(c => !c.has_conflict))
	let canMerge = $derived(mergeRequest?.status === 'pending' && conflictedChanges.length === 0)
	let statusColor = $derived(
		mergeRequest?.status === 'pending' ? 'text-yellow-600 bg-yellow-100' :
		mergeRequest?.status === 'approved' ? 'text-green-600 bg-green-100' :
		mergeRequest?.status === 'merged' ? 'text-blue-600 bg-blue-100' :
		mergeRequest?.status === 'conflicted' ? 'text-red-600 bg-red-100' :
		'text-gray-600 bg-gray-100'
	)
</script>

<svelte:head>
	<title>Merge Request #{mergeRequestId} - {$workspaceStore}</title>
</svelte:head>

{#if loading}
	<div class="container mx-auto p-6 max-w-4xl">
		<div class="flex items-center justify-center p-8">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
		</div>
	</div>
{:else if mergeRequest}
	<div class="container mx-auto p-6 max-w-4xl">
		<!-- Header -->
		<div class="flex items-center gap-4 mb-6">
			<Button variant="border" size="sm" href="{base}/workspace_merge">
				<ArrowLeft size={16} />
				Back to merge overview
			</Button>
			<div class="flex items-center gap-2">
				<GitPullRequest size={20} class="text-blue-500" />
				<h1 class="text-2xl font-bold">Merge Request #{mergeRequestId}</h1>
			</div>
		</div>

		<!-- Merge Request Info -->
		<div class="bg-surface-secondary rounded-lg p-4 mb-6">
			<div class="flex items-center justify-between mb-4">
				<div>
					<h2 class="text-xl font-bold">{mergeRequest.title}</h2>
					<div class="text-sm text-secondary mt-1">
						From <span class="font-mono">{mergeRequest.source_workspace_id}</span> 
						to <span class="font-mono">{mergeRequest.target_workspace_id}</span>
					</div>
				</div>
				<span class={twMerge('px-3 py-1 rounded-full text-sm font-medium', statusColor)}>
					{mergeRequest.status}
				</span>
			</div>

			{#if mergeRequest.description}
				<div class="mb-4">
					<h3 class="font-medium mb-2">Description</h3>
					<p class="text-sm text-secondary whitespace-pre-wrap">{mergeRequest.description}</p>
				</div>
			{/if}

			<div class="flex items-center gap-6 text-sm text-secondary">
				<div>Created by {mergeRequest.created_by}</div>
				<div>Created {new Date(mergeRequest.created_at).toLocaleString()}</div>
				{#if mergeRequest.auto_merge}
					<div class="flex items-center gap-1 text-blue-600">
						<Check size={14} />
						Auto-merge enabled
					</div>
				{/if}
			</div>
		</div>

		<!-- Changes Summary -->
		<div class="grid md:grid-cols-3 gap-4 mb-6">
			<div class="bg-surface-secondary rounded-lg p-4">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-3 h-3 bg-green-500 rounded-full"></div>
					<span class="font-medium">Normal Changes</span>
				</div>
				<div class="text-2xl font-bold">{normalChanges.length}</div>
			</div>

			<div class="bg-surface-secondary rounded-lg p-4">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-3 h-3 bg-red-500 rounded-full"></div>
					<span class="font-medium">Conflicts</span>
				</div>
				<div class="text-2xl font-bold">{conflictedChanges.length}</div>
			</div>

			<div class="bg-surface-secondary rounded-lg p-4">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-3 h-3 bg-blue-500 rounded-full"></div>
					<span class="font-medium">Resolved</span>
				</div>
				<div class="text-2xl font-bold">{resolvedChanges.length}</div>
			</div>
		</div>

		<!-- Conflicts Section -->
		{#if conflictedChanges.length > 0}
			<div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
				<div class="flex items-center gap-2 mb-4">
					<AlertTriangle size={20} class="text-red-500" />
					<h3 class="font-bold text-red-800">Conflicts Requiring Resolution</h3>
				</div>

				<div class="space-y-3">
					{#each conflictedChanges as change}
						<div class="bg-white border border-red-200 rounded p-3">
							<div class="flex items-center justify-between mb-2">
								<div>
									<span class="font-mono text-sm">{change.resource_type}: {change.resource_path}</span>
									<span class="ml-2 px-2 py-1 bg-orange-100 text-orange-800 text-xs rounded">
										{change.change_type}
									</span>
								</div>
								<Button size="xs" on:click={() => loadResourceContent(change)}>
									View Diff
								</Button>
							</div>
							
							{#if change.conflict_reason}
								<div class="text-sm text-red-600 mb-3">{change.conflict_reason}</div>
							{/if}

							<div class="flex gap-2">
								<Button 
									size="xs" 
									variant="border"
									on:click={() => resolveConflict(change.id, 'take_source')}
									disabled={resolvingConflict}
								>
									Take Source
								</Button>
								<Button 
									size="xs" 
									variant="border"
									on:click={() => resolveConflict(change.id, 'take_target')}
									disabled={resolvingConflict}
								>
									Take Target
								</Button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- All Changes -->
		<div class="bg-surface-secondary rounded-lg p-4 mb-6">
			<h3 class="font-bold mb-4">All Changes ({changes.length})</h3>
			
			<div class="space-y-2">
				{#each changes as change}
					<div class="flex items-center justify-between p-3 bg-surface rounded border">
						<div class="flex items-center gap-3">
							<div class={twMerge(
								'w-3 h-3 rounded-full',
								change.has_conflict && !change.resolved ? 'bg-red-500' :
								change.resolved ? 'bg-blue-500' : 'bg-green-500'
							)}></div>
							
							<div>
								<span class="font-mono text-sm">{change.resource_type}: {change.resource_path}</span>
								<div class="flex items-center gap-2 mt-1">
									<span class={twMerge(
										'px-2 py-1 text-xs rounded',
										change.change_type === 'added' ? 'bg-green-100 text-green-800' :
										change.change_type === 'modified' ? 'bg-yellow-100 text-yellow-800' :
										'bg-red-100 text-red-800'
									)}>
										{change.change_type}
									</span>
									
									{#if change.has_conflict}
										{#if change.resolved}
											<span class="px-2 py-1 text-xs bg-blue-100 text-blue-800 rounded">
												Resolved: {change.resolution_strategy}
											</span>
										{:else}
											<span class="px-2 py-1 text-xs bg-red-100 text-red-800 rounded">
												Conflict
											</span>
										{/if}
									{/if}
								</div>
							</div>
						</div>
						
						<Button size="xs" variant="border" on:click={() => loadResourceContent(change)}>
							View
						</Button>
					</div>
				{/each}
			</div>
		</div>

		<!-- Actions -->
		{#if mergeRequest.status === 'pending'}
			<div class="flex gap-4">
				{#if canMerge}
					<Button on:click={executeMerge} disabled={merging}>
						<GitMerge size={16} />
						{merging ? 'Merging...' : 'Execute Merge'}
					</Button>
				{:else}
					<Button disabled title="Resolve all conflicts before merging">
						<AlertTriangle size={16} />
						Resolve Conflicts First
					</Button>
				{/if}
				
				<Button variant="border" disabled={rejecting}>
					<X size={16} />
					Reject Merge Request
				</Button>
			</div>
		{:else if mergeRequest.status === 'merged'}
			<div class="flex items-center gap-2 text-green-600">
				<Check size={20} />
				<span class="font-medium">Merge completed successfully</span>
				{#if mergeRequest.merged_at}
					<span class="text-sm">
						on {new Date(mergeRequest.merged_at).toLocaleString()} 
						by {mergeRequest.merged_by}
					</span>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Diff Modal -->
	{#if showDiff && selectedChange}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
			<div class="bg-surface rounded-lg w-full h-full max-w-6xl max-h-[90vh] m-4 flex flex-col">
				<div class="flex items-center justify-between p-4 border-b">
					<h3 class="font-bold">
						{selectedChange.resource_type}: {selectedChange.resource_path}
					</h3>
					<Button size="sm" variant="border" on:click={() => showDiff = false}>
						<X size={16} />
						Close
					</Button>
				</div>
				
				<div class="flex-1 p-4">
					<div class="h-full border rounded">
						<!-- Placeholder for diff content -->
						<div class="h-full flex items-center justify-center text-secondary">
							<div class="text-center">
								<div class="text-lg mb-2">Diff View</div>
								<div class="text-sm">
									Content comparison would be shown here
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
{/if}