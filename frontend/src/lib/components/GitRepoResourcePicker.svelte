<script lang="ts">
	import { HelpersService, ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent } from './common'
	import { GitBranch, Loader2, FolderOpen } from 'lucide-svelte'
	import Select from './select/Select.svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		open: boolean
		currentResource?: string
		currentCommit?: string
	}

	let {
		open = $bindable(),
		currentResource = undefined,
		currentCommit = undefined
	}: Props = $props()

	const dispatch = createEventDispatcher<{
		selected: {
			resourcePath: string
			playbook?: string
			inventoriesLocation?: string
		}
		addInventories: {
			inventoryPaths: string[]
		}
	}>()

	let drawer: Drawer | undefined = $state(undefined)
	let loading = $state(false)
	let gitRepoResources = $state<{ value: string; label: string }[]>([])
	let selectedResource = $state<string | undefined>(undefined)
	let playbook = $state('')
	let inventoriesLocation = $state('')
	let loadingInventories = $state(false)

	async function loadGitRepoResources() {
		if (!$workspaceStore) return

		loading = true
		try {
			const resources = await ResourceService.listResource({
				workspace: $workspaceStore,
				resourceType: 'git_repository'
			})

			gitRepoResources = resources.map((resource) => ({
				value: resource.path,
				label: resource.path
			}))
		} catch (error) {
			console.error('Failed to load git repository resources:', error)
			gitRepoResources = []
		} finally {
			loading = false
		}
	}

	$effect(() => {
		if (open && $workspaceStore) {
			loadGitRepoResources()
			// Set current resource as selected when opening
			selectedResource = currentResource
			drawer?.openDrawer?.()
		} else if (!open) {
			drawer?.closeDrawer?.()
		}
	})

	function handleSelect() {
		if (selectedResource) {
			dispatch('selected', { resourcePath: selectedResource, playbook, inventoriesLocation })
			selectedResource = undefined
			open = false
		}
	}

	function handleClose() {
		selectedResource = undefined
		playbook = ''
		inventoriesLocation = ''
		loadingInventories = false
		open = false
	}

	export async function getInventoryFiles(
		resourcePath: string,
		inventoriesPath: string,
		commitHash: string
	): Promise<string[]> {
		const rootPath = `gitrepos/${$workspaceStore}/${resourcePath}/${commitHash}/`

		if (inventoriesPath.startsWith('./')) inventoriesPath = inventoriesPath.slice(2)

		let files = await HelpersService.listGitRepoFiles({
			workspace: $workspaceStore!,
			maxKeys: 100,
			marker: undefined,
			prefix: `${rootPath}/${inventoriesPath}`
		})

		console.log(rootPath, inventoriesPath, files)

		const fileNames = files.windmill_large_files.map((f) =>
			f.s3.slice(rootPath.length)
		)

		// Return dummy data for testing
		return fileNames
	}
	async function handleAddInventories() {
		if (!selectedResource || !inventoriesLocation.trim()) return

		loadingInventories = true
		try {
			// Get commit hash if not provided
			let commitHash = currentCommit
			if (!commitHash) {
				try {
					const result = await ResourceService.getGitCommitHash({
						workspace: $workspaceStore!,
						path: selectedResource
					})
					commitHash = result.commit_hash
				} catch (err) {
					console.error('Failed to get commit hash:', err)
					throw new Error('Could not get commit hash for repository')
				}
			}

			if (!commitHash) {
				throw new Error('No commit hash available')
			}

			const inventoryFiles = await getInventoryFiles(
				selectedResource,
				inventoriesLocation.trim(),
				commitHash
			)

			// Dispatch event to update the script with additional_inventories
			dispatch('addInventories', {
				inventoryPaths: inventoryFiles
			})

			// TODO: Add success feedback
			console.log('Added inventories:', inventoryFiles)
		} catch (error) {
			console.error('Failed to load inventory files:', error)
			// TODO: Add error feedback
		} finally {
			loadingInventories = false
		}
	}
</script>

<Drawer bind:this={drawer} size="600px">
	<DrawerContent
		title="Select Git Repository Resource"
		on:close={handleClose}
		tooltip="Select a git repository resource to delegate ansible execution to"
	>
		<div class="flex flex-col gap-4 p-4">
			<div class="flex flex-col gap-2">
				<div class="text-sm font-medium text-primary"> Git Repository Resource </div>

				{#if currentResource}
					<div
						class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700 rounded-md"
					>
						<div class="flex items-center gap-2">
							<GitBranch size={16} class="text-blue-600 dark:text-blue-400 flex-shrink-0" />
							<div class="flex-1 min-w-0">
								<div class="text-sm font-medium text-blue-800 dark:text-blue-200">
									Currently delegating to:
								</div>
								<div class="text-sm text-blue-600 dark:text-blue-300 truncate">
									{currentResource}
								</div>
							</div>
						</div>
					</div>
				{/if}

				{#if loading}
					<div class="flex items-center gap-2 p-2">
						<Loader2 size={16} class="animate-spin" />
						<span class="text-sm text-secondary">Loading git repository resources...</span>
					</div>
				{:else if gitRepoResources.length === 0}
					<div class="p-4 text-center border border-gray-200 dark:border-gray-700 rounded-md">
						<GitBranch size={24} class="mx-auto mb-2 text-tertiary" />
						<p class="text-sm text-secondary mb-2">No git repository resources found</p>
						<p class="text-xs text-tertiary">
							Create a git repository resource first to use this feature
						</p>
					</div>
				{:else}
					<Select
						bind:value={selectedResource}
						items={gitRepoResources}
						placeholder="Select a git repository resource..."
						class="w-full"
					/>
				{/if}
			</div>

			<!-- Playbook Configuration -->
			<div class="flex flex-col gap-2">
				<div class="text-sm font-medium text-primary">
					Playbook Path
					<span class="text-xs text-tertiary font-normal ml-1">(optional)</span>
				</div>
				<input
					type="text"
					bind:value={playbook}
					placeholder="e.g., ./playbooks/site.yml"
					class="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary placeholder-tertiary focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				/>
				<p class="text-xs text-tertiary">
					Specify the path to your main playbook file relative to the git repository root
				</p>
			</div>

			<!-- Inventories Location Configuration -->
			<div class="flex flex-col gap-2">
				<div class="text-sm font-medium text-primary">
					Inventories Location
					<span class="text-xs text-tertiary font-normal ml-1">(optional)</span>
				</div>
				<input
					type="text"
					bind:value={inventoriesLocation}
					placeholder="e.g., hosts"
					class="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary placeholder-tertiary focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				/>
				<p class="text-xs text-tertiary">
					Specify the directory containing your inventory files relative to the git repository root
				</p>

				{#if inventoriesLocation.trim() && selectedResource}
					<div class="mt-2">
						<Button
							color="light"
							variant="border"
							size="xs"
							disabled={loadingInventories}
							startIcon={{ icon: loadingInventories ? Loader2 : FolderOpen }}
							onclick={handleAddInventories}
							btnClasses={loadingInventories ? 'animate-pulse' : ''}
						>
							{#if loadingInventories}
								Loading inventories...
							{:else}
								Add available inventories to script as options
							{/if}
						</Button>
					</div>
				{/if}
			</div>

			<div class="flex justify-end gap-2 pt-4 border-t border-gray-200 dark:border-gray-700">
				<Button color="light" variant="border" on:click={handleClose}>Cancel</Button>
				<Button
					color="dark"
					disabled={!selectedResource || loading}
					on:click={handleSelect}
					startIcon={{ icon: GitBranch }}
				>
					Apply Configuration
				</Button>
			</div>
		</div>
	</DrawerContent>
</Drawer>
