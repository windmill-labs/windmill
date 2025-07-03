<script lang="ts">
	import { workspaceStore, userStore } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { onMount } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { GitBranch, GitMerge, GitPullRequest, ArrowLeft, FileText, AlertTriangle, Check, X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	let workspaceForkInfo = $state<{ parent_workspace_id: string; fork_workspace_id: string; created_at: string; created_by: string } | null>(null)
	let pendingMergeRequests = $state<any[]>([])
	let resourceRefs = $state<any[]>([])
	let loading = $state(true)
	let creating = $state(false)

	// Create merge request form
	let showCreateForm = $state(false)
	let mergeTitle = $state('')
	let mergeDescription = $state('')
	let autoMerge = $state(false)

	async function loadMergeData() {
		if (!$workspaceStore || !$userStore?.is_admin) {
			goto(`${base}/`)
			return
		}

		loading = true
		try {
			// Check if current workspace is a fork
			const forkResponse = await fetch(`/api/w/${$workspaceStore}/workspaces/fork/fork_info`)
			if (forkResponse.ok) {
				workspaceForkInfo = await forkResponse.json()
			} else {
				sendUserToast('This workspace is not a fork. Redirecting...', true)
				goto(`${base}/`)
				return
			}

			// Load pending merge requests
			const mergeResponse = await fetch(`/api/w/${$workspaceStore}/workspaces/merge/list_merge_requests`)
			if (mergeResponse.ok) {
				pendingMergeRequests = await mergeResponse.json()
			}

			// Load resource references to show what's changed
			const refsResponse = await fetch(`/api/w/${$workspaceStore}/workspaces/fork/resource_refs`)
			if (refsResponse.ok) {
				resourceRefs = await refsResponse.json()
			}

		} catch (error) {
			sendUserToast('Failed to load merge data', true)
			console.error(error)
		} finally {
			loading = false
		}
	}

	async function createMergeRequest() {
		if (!workspaceForkInfo || !mergeTitle.trim()) return

		creating = true
		try {
			const response = await fetch(`/api/w/${$workspaceStore}/workspaces/merge/create_merge_request`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					title: mergeTitle,
					description: mergeDescription || undefined,
					auto_merge: autoMerge
				})
			})

			if (response.ok) {
				const result = await response.json()
				sendUserToast(`Merge request created: ${result.merge_request.title}`)
				showCreateForm = false
				mergeTitle = ''
				mergeDescription = ''
				autoMerge = false
				await loadMergeData()
			} else {
				const error = await response.text()
				sendUserToast(`Failed to create merge request: ${error}`, true)
			}
		} catch (error) {
			sendUserToast('Failed to create merge request', true)
			console.error(error)
		} finally {
			creating = false
		}
	}

	onMount(loadMergeData)

	// Computed values
	let changedResources = $derived(resourceRefs.filter(ref => !ref.is_reference))
	let referencedResources = $derived(resourceRefs.filter(ref => ref.is_reference))
	let hasChanges = $derived(changedResources.length > 0)

	// Automatically set merge title based on changes
	$effect(() => {
		if (hasChanges && !mergeTitle) {
			mergeTitle = `Merge ${changedResources.length} changes from ${$workspaceStore}`
		}
	})
</script>

<svelte:head>
	<title>Workspace Merge - {$workspaceStore}</title>
</svelte:head>

{#if loading}
	<CenteredModal title="Loading...">
		<div class="flex items-center justify-center p-8">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
		</div>
	</CenteredModal>
{:else if workspaceForkInfo}
	<div class="container mx-auto p-6 max-w-4xl">
		<!-- Header -->
		<div class="flex items-center gap-4 mb-6">
			<Button variant="border" size="sm" on:click={() => goto(base + '/')}>
				<ArrowLeft size={16} />
				Back to workspace
			</Button>
			<div class="flex items-center gap-2">
				<GitBranch size={20} class="text-blue-500" />
				<h1 class="text-2xl font-bold">Workspace Merge</h1>
			</div>
		</div>

		<!-- Fork Info -->
		<div class="bg-surface-secondary rounded-lg p-4 mb-6">
			<div class="flex items-center gap-2 mb-2">
				<GitBranch size={16} class="text-blue-500" />
				<span class="font-medium">Fork Information</span>
			</div>
			<div class="text-sm text-secondary">
				<div>Forked from: <span class="font-mono text-blue-600">{workspaceForkInfo.parent_workspace_id}</span></div>
				<div>Created by: {workspaceForkInfo.created_by}</div>
				<div>Created at: {new Date(workspaceForkInfo.created_at).toLocaleString()}</div>
			</div>
		</div>

		<!-- Changes Summary -->
		<div class="bg-surface-secondary rounded-lg p-4 mb-6">
			<div class="flex items-center gap-2 mb-4">
				<FileText size={16} />
				<span class="font-medium">Changes Summary</span>
			</div>
			
			{#if hasChanges}
				<div class="grid md:grid-cols-2 gap-4">
					<div class="space-y-2">
						<div class="flex items-center gap-2 text-orange-600">
							<div class="w-3 h-3 bg-orange-500 rounded-full"></div>
							<span class="font-medium">{changedResources.length} Modified Resources</span>
						</div>
						<div class="max-h-32 overflow-y-auto space-y-1">
							{#each changedResources as resource}
								<div class="text-sm text-secondary font-mono pl-5">
									{resource.resource_type}: {resource.resource_path}
								</div>
							{/each}
						</div>
					</div>
					
					<div class="space-y-2">
						<div class="flex items-center gap-2 text-blue-600">
							<div class="w-3 h-3 bg-blue-500 rounded-full"></div>
							<span class="font-medium">{referencedResources.length} Referenced Resources</span>
						</div>
						<div class="text-sm text-secondary">
							These resources are unchanged and still reference the parent workspace.
						</div>
					</div>
				</div>
			{:else}
				<div class="text-center py-4 text-secondary">
					<AlertTriangle size={24} class="mx-auto mb-2 text-yellow-500" />
					<div>No changes to merge</div>
					<div class="text-sm">All resources are still referencing the parent workspace</div>
				</div>
			{/if}
		</div>

		<!-- Pending Merge Requests -->
		{#if pendingMergeRequests.length > 0}
			<div class="bg-surface-secondary rounded-lg p-4 mb-6">
				<div class="flex items-center gap-2 mb-4">
					<GitPullRequest size={16} class="text-blue-500" />
					<span class="font-medium">Pending Merge Requests</span>
				</div>
				
				<div class="space-y-3">
					{#each pendingMergeRequests as request}
						<div class="flex items-center justify-between p-3 bg-surface rounded border">
							<div>
								<div class="font-medium">{request.title}</div>
								<div class="text-sm text-secondary">
									Created by {request.created_by} • {new Date(request.created_at).toLocaleDateString()}
								</div>
								{#if request.description}
									<div class="text-sm text-tertiary mt-1">{request.description}</div>
								{/if}
							</div>
							<div class="flex items-center gap-2">
								<span class={twMerge(
									'px-2 py-1 rounded text-xs font-medium',
									request.status === 'pending' ? 'bg-yellow-100 text-yellow-800' :
									request.status === 'approved' ? 'bg-green-100 text-green-800' :
									request.status === 'merged' ? 'bg-blue-100 text-blue-800' :
									'bg-red-100 text-red-800'
								)}>
									{request.status}
								</span>
								<Button size="xs" href="{base}/workspace_merge/{request.id}">
									View
								</Button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Actions -->
		<div class="flex gap-4">
			{#if hasChanges && !showCreateForm}
				<Button on:click={() => showCreateForm = true}>
					<GitMerge size={16} />
					Create Merge Request
				</Button>
			{/if}
			
			<Button variant="border" href="{base}/workspace_settings">
				Workspace Settings
			</Button>
		</div>

		<!-- Create Merge Request Form -->
		{#if showCreateForm}
			<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
				<div class="bg-surface rounded-lg p-6 max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto">
					<h3 class="text-lg font-bold mb-4">Create Merge Request</h3>
					
					<div class="space-y-4">
						<div>
							<label class="block text-sm font-medium mb-2">Title</label>
							<input
								type="text"
								bind:value={mergeTitle}
								class="w-full p-2 border rounded"
								placeholder="Describe your changes..."
							/>
						</div>
						
						<div>
							<label class="block text-sm font-medium mb-2">Description (optional)</label>
							<textarea
								bind:value={mergeDescription}
								class="w-full p-2 border rounded h-24"
								placeholder="Additional details about the changes..."
							></textarea>
						</div>
						
						<div class="flex items-center gap-2">
							<input type="checkbox" bind:checked={autoMerge} id="auto-merge" />
							<label for="auto-merge" class="text-sm">Auto-merge if no conflicts</label>
						</div>
						
						<div class="bg-surface-secondary p-3 rounded text-sm">
							<div class="font-medium mb-1">Changes to be merged:</div>
							<ul class="list-disc list-inside space-y-1">
								{#each changedResources.slice(0, 5) as resource}
									<li class="font-mono text-xs">{resource.resource_type}: {resource.resource_path}</li>
								{/each}
								{#if changedResources.length > 5}
									<li class="text-secondary">...and {changedResources.length - 5} more</li>
								{/if}
							</ul>
						</div>
					</div>
					
					<div class="flex gap-3 mt-6">
						<Button 
							on:click={createMergeRequest} 
							disabled={creating || !mergeTitle.trim()}
						>
							{creating ? 'Creating...' : 'Create Merge Request'}
						</Button>
						<Button 
							variant="border" 
							on:click={() => showCreateForm = false}
							disabled={creating}
						>
							Cancel
						</Button>
					</div>
				</div>
			</div>
		{/if}
	</div>
{/if}