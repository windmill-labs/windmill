<script lang="ts">
	import { Skeleton } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString } from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { createEventDispatcher } from 'svelte'

	// export let failureModule: boolean
	const dispatch = createEventDispatcher()

	let items: Flow[] | undefined = undefined
	let filteredItems: (Flow & { marked?: string })[] | undefined = undefined
	export let filter = ''
	$: $workspaceStore && loadFlows()

	let ownerFilter: string | undefined = undefined
	$: prefilteredItems = ownerFilter ? items?.filter((x) => x.path.startsWith(ownerFilter!)) : items

	export let owners: string[] = []
	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	async function loadFlows() {
		items = await FlowService.listFlows({ workspace: $workspaceStore! })
	}
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>
<div class="flex flex-col min-h-0">
	{#if filteredItems}
		{#if filter.length > 0 && filteredItems.length == 0}
			<div class="text-2xs text-tercary font-extralight text-center py-2 px-3 items-center">
				No items found.
			</div>
		{/if}
		<ul class="overflow-auto">
			{#each filteredItems as { path, summary, marked }}
				<li class="flex flex-row w-full">
					<button
						class="px-3 py-2 gap-2 flex flex-row w-full hover:bg-surface-hover bg-surface transition-all items-center rounded-md text-left text-2xs text-primary font-normal"
						on:click={async () => {
							dispatch('pickFlow', {
								path,
								summary
							})
						}}
					>
						<BarsStaggered size={14} />
						<span class="grow truncate">
							{#if marked}
								{@html marked}
							{:else}
								{!summary || summary.length == 0 ? path : summary}
							{/if}
						</span>
					</button>
				</li>
			{/each}
		</ul>
	{:else}
		{#each Array(10).fill(0) as _}
			<Skeleton layout={[0.5, [1.5]]} />
		{/each}
	{/if}
</div>
