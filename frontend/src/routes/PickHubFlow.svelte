<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte'
	import { Badge, Skeleton } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { loadHubFlows } from '$lib/utils'

	export let filter = ''

	type Item = { apps: string[]; summary: string }
	let hubFlows: any[] | undefined = undefined
	let filteredItems: (Item & { marked?: string })[] = []
	let appFilter: string | undefined = undefined

	$: prefilteredItems = appFilter
		? (hubFlows ?? []).filter((i) => i.apps.includes(appFilter))
		: hubFlows ?? []

	$: apps = Array.from(new Set(filteredItems?.flatMap((x) => x.apps) ?? [])).sort()

	const dispatch = createEventDispatcher()

	onMount(async () => {
		hubFlows = await loadHubFlows()
	})
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => x.summary + ' (' + x.apps.join(', ') + ')'}
/>

<div class="flex flex-col min-h-0">
	<div class="w-12/12 pb-2 flex flex-row my-1 gap-1">
		<input type="text" placeholder="Search Hub Flows" bind:value={filter} class="text-2xl grow" />
	</div>

	<div class="gap-2 w-full flex flex-wrap pb-2">
		{#each apps as app}
			<Badge
				class="cursor-pointer hover:bg-gray-200"
				on:click={() => {
					appFilter = appFilter == app ? undefined : app
				}}
				capitalize
				color={app === appFilter ? 'blue' : 'gray'}
			>
				{app}
				{#if app === appFilter}&cross;{/if}
			</Badge>
		{/each}
	</div>
	<div class="overflow-auto">
		<ul class="divide-y divide-gray-200">
			{#if hubFlows}
				{#if filter.length > 0 && filteredItems.length == 0}
					<p>No items found</p>
				{/if}
				{#each filteredItems as obj}
					<li class="flex flex-row w-full">
						<button
							class="py-4 px-1 gap-1 flex flex-row grow hover:bg-blue-50 bg-white transition-all"
							on:click={() => {
								dispatch('pick', obj)
							}}
						>
							<div class="mr-2 text-sm text-left w-32 shrink-0">{obj.apps.join(', ')}</div>
							<div class="mr-2 text-left">
								{#if obj.marked}
									{@html obj.marked ?? ''}
								{:else}
									{obj.summary ?? ''}
								{/if}
							</div>
						</button>
					</li>
				{/each}
			{:else}
				{#each Array(10).fill(0) as sk}
					<Skeleton layout={[[4], 0.5]} />
				{/each}
			{/if}
		</ul>
	</div>
</div>
