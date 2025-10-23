<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Popover } from './meltComponents'
	import { GitBranch, Loader2 } from 'lucide-svelte'
	import { createEventDispatcher, untrack } from 'svelte'

	interface Props {
		isOpen?: boolean
		children?: import('svelte').Snippet
	}

	let { isOpen = $bindable(false), children }: Props = $props()

	const dispatch = createEventDispatcher<{
		selected: { resourcePath: string }
	}>()

	let loading = $state(false)
	let gitRepoResources = $state<{ path: string; description?: string }[]>([])

	async function loadGitRepoResources() {
		if (!$workspaceStore || loading) return

		loading = true
		try {
			const resources = await ResourceService.listResource({
				workspace: $workspaceStore,
				resourceType: 'git_repository'
			})

			gitRepoResources = resources
		} catch (error) {
			console.error('Failed to load git repository resources:', error)
			gitRepoResources = []
		} finally {
			loading = false
		}
	}

	$effect(() => {
		if (isOpen && $workspaceStore) {
			untrack(() => loadGitRepoResources())
		}
	})

	function handleSelect(resourcePath: string) {
		dispatch('selected', { resourcePath })
		isOpen = false
	}
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
	usePointerDownOutside
	closeOnOtherPopoverOpen
	bind:isOpen
	escapeBehavior="ignore"
>
	{#snippet trigger()}
		{@render children?.()}
	{/snippet}
	{#snippet content()}
		<div class="w-64 max-h-72 overflow-y-auto">
			{#if loading}
				<div class="flex items-center gap-2 p-3">
					<Loader2 size={16} class="animate-spin" />
					<span class="text-sm text-secondary">Loading...</span>
				</div>
			{:else if gitRepoResources.length === 0}
				<div class="p-4 text-center">
					<GitBranch size={20} class="mx-auto mb-2 text-primary" />
					<p class="text-sm text-secondary mb-1">No git repositories found</p>
					<p class="text-xs text-primary">Create a git repository resource first</p>
				</div>
			{:else}
				<div class="py-1">
					{#each gitRepoResources as resource}
						<button
							class="w-full text-left px-3 py-2 hover:bg-surface-hover text-sm flex items-center gap-2 transition-colors"
							onclick={() => handleSelect(resource.path)}
						>
							<GitBranch size={14} class="text-primary flex-shrink-0" />
							<div class="flex-1 min-w-0">
								<div class="font-medium text-primary truncate">{resource.path}</div>
								{#if resource.description}
									<div class="text-xs text-primary truncate">{resource.description}</div>
								{/if}
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/snippet}
</Popover>
