<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher, onMount } from 'svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { ResourceService, type ListableResource } from '$lib/gen'
	import { emptyString } from '$lib/utils'
	import { Skeleton } from '$lib/components/common'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		filter?: string
		children?: import('svelte').Snippet
	}

	let { filter = $bindable(''), children }: Props = $props()

	let agents: ListableResource[] | undefined = $state(undefined)
	let filteredItems: (ListableResource & { marked?: string })[] = $state([])
	let prefilteredItems = $derived(agents ?? [])

	const dispatch = createEventDispatcher()

	onMount(async () => {
		agents = await ResourceService.listResource({
			workspace: $operatingWorkspace!,
			resourceType: 'ai_agent',
			perPage: 300
		})
	})
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.description) ? x.path : x.description + ' (' + x.path + ')')}
/>
<div class="w-full flex mt-1 items-center gap-2">
	{@render children?.()}
	<input
		type="text"
		placeholder="Search workspace AI agents"
		onkeydown={stopPropagation(bubble('keydown'))}
		bind:value={filter}
		class="text-2xl grow mb-4"
	/>
</div>

{#if agents}
	{#if filteredItems.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md">
			{#each filteredItems as item (item)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow justify-between hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
						onclick={() => dispatch('pick', item.path)}
					>
						<div class="flex items-center gap-4">
							<RowIcon kind="agent" />

							<div class="w-full text-left font-normal">
								<div class="text-primary flex-wrap text-md font-semibold mb-1">
									{#if item.marked}
										{@html item.marked ?? ''}
									{:else}
										{emptyString(item.description) ? item.path : item.description}
									{/if}
								</div>
								<div class="text-primary text-xs">
									{item.path}
								</div>
							</div>
						</div>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[[4], 0.5]} />
	{/each}
{/if}
