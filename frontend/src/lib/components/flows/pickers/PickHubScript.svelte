<script lang="ts">
	import { hubScripts } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import type { HubItem } from './model'
	import Fuse from 'fuse.js'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import { Badge } from '$lib/components/common'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'

	let items: HubItem[] = []

	let filteredItems: Item[] | undefined = []
	let itemsFilter = ''
	let appFilter: string | undefined = undefined

	const fuseOptions = {
		includeScore: false,
		keys: ['path', 'summay']
	}
	const fuse: Fuse<Item> = new Fuse(items, fuseOptions)

	$: {
		items =
			$hubScripts?.filter(
				(x) => x.kind == kind && (appFilter == undefined || x.app == appFilter)
			) ?? []
		fuse.setCollection(items)
	}

	$: filteredItems =
		itemsFilter.length > 0 && items ? fuse.search(itemsFilter).map((value) => value.item) : items

	$: apps = Array.from(new Set(filteredItems?.map((x) => x.app) ?? []))

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col min-h-0">
	<div class="w-12/12 pb-2">
		<input type="text" placeholder="Search script" bind:value={itemsFilter} class="search-item" />
	</div>

	<div class="gap-2 w-full flex flex-wrap pb-2">
		{#each apps as app}
			<Badge
				class="cursor-pointer"
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
	{#if filteredItems}
		<div class="overflow-auto">
			<ul class="divide-y divide-gray-200">
				{#each filteredItems as obj}
					<li class="flex flex-row w-full">
						<button
							class="py-4 px-1 gap-1 flex flex-row grow hover:bg-blue-50 bg-white transition-all"
							on:click={() => {
								dispatch('pick', obj)
							}}
						>
							<div class="mr-2 text-sm text-left truncate w-36  shrink-0">
								<IconedResourceType after={true} silent={false} name={obj['app']} />
							</div>
							<div class="mr-2 text-left">{obj['summary'] ?? ''}</div>
						</button>
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>
