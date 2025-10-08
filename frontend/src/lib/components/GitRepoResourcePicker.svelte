<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent } from './common'
	import { GitBranch, Loader2 } from 'lucide-svelte'
	import Select from './select/Select.svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		open: boolean
	}

	let {
		open = $bindable()
	}: Props = $props()

	const dispatch = createEventDispatcher<{
		selected: { resourcePath: string }
	}>()

	let drawer: Drawer | undefined = $state(undefined)
	let loading = $state(false)
	let gitRepoResources = $state<{ value: string; label: string }[]>([])
	let selectedResource = $state<string | undefined>(undefined)

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
			drawer?.openDrawer?.()
		} else if (!open) {
			drawer?.closeDrawer?.()
		}
	})

	function handleSelect() {
		if (selectedResource) {
			dispatch('selected', { resourcePath: selectedResource })
			selectedResource = undefined
			open = false
		}
	}

	function handleClose() {
		selectedResource = undefined
		open = false
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
				<label class="text-sm font-medium text-primary">
					Git Repository Resource
				</label>
				
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

			<div class="flex justify-end gap-2 pt-4 border-t border-gray-200 dark:border-gray-700">
				<Button
					color="light"
					variant="border"
					on:click={handleClose}
				>
					Cancel
				</Button>
				<Button
					color="dark"
					disabled={!selectedResource || loading}
					on:click={handleSelect}
					startIcon={{ icon: GitBranch }}
				>
					Select Resource
				</Button>
			</div>
		</div>
	</DrawerContent>
</Drawer>