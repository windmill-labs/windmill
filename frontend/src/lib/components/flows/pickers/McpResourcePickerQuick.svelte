<script module lang="ts">
	let initialWorkspace = get(workspaceStore)
	let loadResourcesCached = createCache(
		({ workspace }: { workspace?: string; refreshCount?: number }) =>
			workspace && get(userStore)
				? ResourceService.listResource({ workspace, resourceType: 'mcp' })
				: undefined,
		initialWorkspace
			? {
					initial: {
						workspace: initialWorkspace,
						refreshCount: 0
					},
					invalidateMs: 1000 * 60
				}
			: {}
	)
</script>

<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, untrack } from 'svelte'
	import { ResourceService } from '$lib/gen'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { Skeleton } from '$lib/components/common'
	import { createCache, emptyString } from '$lib/utils'
	import { Plug } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { get } from 'svelte/store'
	import { userStore } from '$lib/stores'

	type McpResource = {
		path: string
		description?: string
		resource_type: string
	}

	let resources = usePromise(
		async () => await loadResourcesCached({ workspace: $workspaceStore!, refreshCount }),
		{ loadInit: false, clearValueOnRefresh: false }
	)

	let filteredResources: (McpResource & { marked?: string })[] | undefined = $state(undefined)

	interface Props {
		selected?: number | undefined
		displayPath?: boolean
		filter?: string
		refreshCount?: number
	}

	let { selected = undefined, displayPath = false, filter = '', refreshCount = 0 }: Props = $props()

	const dispatch = createEventDispatcher()

	function onKeyDown(e: KeyboardEvent) {
		if (
			selected != undefined &&
			filteredResources &&
			selected >= 0 &&
			selected < filteredResources.length &&
			e.key === 'Enter'
		) {
			e.preventDefault()
			let resource = filteredResources[selected]
			dispatch('pickMcpResource', { path: resource.path })
		}
	}

	$effect(() => {
		refreshCount
		$workspaceStore && untrack(() => resources.refresh())
	})
</script>

<SearchItems
	{filter}
	items={resources.value}
	bind:filteredItems={filteredResources}
	f={(x) => (emptyString(x.description) ? x.path : x.description + ' (' + x.path + ')')}
/>

<svelte:window onkeydown={onKeyDown} />
{#if filteredResources}
	{#if filteredResources.length == 0}
		<div class="text-2xs text-tertiary font-light text-center py-2 px-3 items-center">
			No MCP resources found.
		</div>
	{/if}
	<ul>
		{#each filteredResources as { path, description, marked }, index}
			<li class="w-full">
				<Popover class="w-full" placement="right" forceOpen={index === selected}>
					{#snippet text()}
						<div class="flex flex-col">
							<div class="text-left text-xs font-normal leading-tight py-0">
								{description ?? ''}
							</div>
							<div class="text-left text-2xs font-normal">
								{path ?? ''}
							</div>
						</div>
					{/snippet}
					<button
						class="px-3 py-2 gap-2 flex flex-row w-full hover:bg-surface-hover transition-all items-center rounded-md {index ===
						selected
							? 'bg-surface-hover'
							: ''}"
						onclick={() => {
							dispatch('pickMcpResource', { path: path })
						}}
					>
						<Plug size={14} />
						<div class="flex flex-col grow min-w-0">
							<div class="grow min-w-0 truncate text-left text-2xs text-primary font-normal">
								{#if marked}
									{@html marked}
								{:else}
									{!description || description.length == 0 ? path : description}
								{/if}
							</div>
							{#if displayPath && path}
								<div class="grow min-w-0 truncate text-left text-2xs text-secondary font-[220]">
									{path}
								</div>
							{/if}
						</div>
						{#if index === selected}
							<kbd class="!text-xs">&crarr;</kbd>
						{/if}
					</button>
				</Popover>
			</li>
		{/each}
	</ul>
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [1.5]]} />
	{/each}
{/if}
