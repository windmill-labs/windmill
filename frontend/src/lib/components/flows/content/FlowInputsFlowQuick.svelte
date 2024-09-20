<script lang="ts">
	import { Skeleton } from '$lib/components/common'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString } from '$lib/utils'

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
			<NoItemFound />
		{/if}
		<ul class="overflow-auto">
			{#each filteredItems as { path, summary, marked }}
				<li class="flex flex-row w-full">
					<button
						class="p-1 gap-1 flex flex-row grow hover:bg-surface-hover bg-surface transition-all items-center"
						on:click={async () => {
							dispatch('pickFlow', {
								path,
								summary
							})
						}}
					>
						<div class="w-full max-w-60 text-left text-xs text-primary font-semibold mb-1 truncate">
							{#if marked}
								{@html marked}
							{:else}
								{!summary || summary.length == 0 ? path : summary}
							{/if}
						</div>
					</button>
				</li>
			{/each}
		</ul>
	{:else}
		<div class="mt-6" />

		{#each new Array(6) as _}
			<Skeleton layout={[[4], 0.7]} />
		{/each}
	{/if}
</div>
