<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import type { HubItem } from './model'
	import Fuse from 'fuse.js'
	import { ScriptService } from '$lib/gen'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'

	let items: HubItem[] = []

	let filteredItems: Item[] | undefined = []
	let itemsFilter = ''

	const fuseOptions = {
		includeScore: false,
		keys: ['path', 'summay']
	}
	const fuse: Fuse<Item> = new Fuse(items, fuseOptions)

	$: $workspaceStore && loadItems()

	$: filteredItems =
		itemsFilter.length > 0 && items ? fuse.search(itemsFilter).map((value) => value.item) : items

	async function loadItems(): Promise<void> {
		items = await ScriptService.listScripts({ workspace: $workspaceStore!, kind })
		fuse.setCollection(items)
	}

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col min-h-0">
	<div class="w-12/12 pb-4">
		<input type="text" placeholder="Search script" bind:value={itemsFilter} class="search-item" />
	</div>

	{#if filteredItems}
		<ul class="divide-y divide-gray-200 overflow-auto">
			{#each filteredItems as obj}
				<li class="flex flex-row w-full">
					<button
						class="py-4 px-1 gap-1 flex flex-row grow hover:bg-white hover:border text-black"
						on:click={() => {
							dispatch('pick', obj)
						}}
					>
						<div class="flex flex-col">
							<div class="text-sm font-semibold flex flex-col">
								<span class="mr-2 text-left">{obj['summary'] ?? ''}</span>
								<span class="font-normal text-xs text-left italic overflow-hidden"
									>{obj['path'] ?? ''}</span
								>
							</div>
							<div class="text-xs font-light italic text-left">{obj['description'] ?? ''}</div>
						</div>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>
