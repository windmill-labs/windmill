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
		currentInventories?: string
		currentPlaybook?: string
		gitSshIdentity?: string[]
	}

	let {
		open = $bindable(),
		currentResource = undefined,
		currentCommit = undefined,
		currentInventories = undefined,
		currentPlaybook = undefined,
		gitSshIdentity = undefined
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
	let playbook = $derived(currentPlaybook ?? '')
	let inventoriesLocation = $derived(currentInventories ?? '')
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

		const fileNames = files.windmill_large_files.map((f) => f.s3.slice(rootPath.length))

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
						path: selectedResource,
						gitSshIdentity: gitSshIdentity?.join(',')
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
			<div class="flex flex-col gap-1">
				<div class="text-xs font-semibold text-emphasis"> Git Repository Resource </div>

				{#if currentResource}
					<div
						class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700 rounded-md"
					>
						<div class="flex items-center gap-2">
							<GitBranch size={16} class="text-blue-600 dark:text-blue-400 flex-shrink-0" />
							<div class="flex-1 min-w-0">
								<div class="text-xs font-semibold text-blue-800 dark:text-blue-200">
									Currently delegating to:
								</div>
								<div class="text-xs text-blue-600 dark:text-blue-300 truncate">
									{currentResource}
								</div>
							</div>
						</div>
					</div>
				{/if}

				{#if loading}
					<div class="flex items-center gap-2 p-2">
						<Loader2 size={16} class="animate-spin" />
						<span class="text-xs text-primary">Loading git repository resources...</span>
					</div>
				{:else if gitRepoResources.length === 0}
					<div class="p-4 text-center border border-border-light rounded-md">
						<GitBranch size={24} class="mx-auto mb-2 text-primary" />
						<p class="text-xs text-primary mb-2">No git repository resources found</p>
						<p class="text-2xs text-secondary">
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
			<div class="flex flex-col gap-1">
				<div class="text-xs font-semibold text-emphasis">
					Playbook Path
					<span class="text-xs text-primary font-normal ml-1">(optional)</span>
				</div>
				<p class="text-xs text-primary">
					Specify the path to your main playbook file relative to the git repository root
				</p>
				<input
					type="text"
					bind:value={playbook}
					placeholder="e.g., ./playbooks/site.yml"
					class="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary placeholder-tertiary focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				/>
			</div>

			<!-- Inventories Location Configuration -->
			<div class="flex justify-end gap-2 pt-4 border-t border-gray-200 dark:border-gray-700">
				<Button variant="default" on:click={handleClose}>Cancel</Button>
				<Button
					variant="accent"
					disabled={!selectedResource || loading}
					on:click={handleSelect}
					startIcon={{ icon: GitBranch }}
				>
					Apply Configuration
				</Button>
			</div>
			{#if selectedResource}
				<div class="flex flex-col gap-2">
					<div class="text-xs font-semibold text-emphasis"> Inventories Location </div>
					<input
						type="text"
						bind:value={inventoriesLocation}
						placeholder="e.g., hosts"
						class="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary placeholder-tertiary focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
					/>
					<p class="text-xs text-primary">
						Specify the directory containing your inventory files relative to the git repository
						root
					</p>

					<div class="mt-2">
						<Button
							variant="default"
							unifiedSize="md"
							disabled={loadingInventories || !inventoriesLocation.trim()}
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
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
