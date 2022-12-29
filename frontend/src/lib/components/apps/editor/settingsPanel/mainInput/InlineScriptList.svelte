<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'

	export let filter = ''
	export let inlineScripts: string[] = []

	type Item = { title: string }
	let filteredItems: (Item & { marked?: string })[] = []
	$: items = inlineScripts.map((x) => ({ title: x }))
	$: prefilteredItems = items ?? []

	const dispatch = createEventDispatcher()
</script>

<SearchItems {filter} items={prefilteredItems} bind:filteredItems f={(x) => x.summary} />
<div class="w-full flex mt-1 items-center gap-2">
	<slot />
	<input
		type="text"
		placeholder="Search inline scripts"
		bind:value={filter}
		class="text-2xl grow mb-4"
	/>
</div>

{#if inlineScripts.length === 0}
	<div class="flex flex-col  w-full h-full">
		<div class="text-md">No detached inline scripts</div>
	</div>
{:else if filteredItems.length === 0}
	<NoItemFound />
{:else}
	<ul class="divide-y divide-gray-200 border rounded-md">
		{#each filteredItems as item (item)}
			<li class="flex flex-row w-full">
				<button
					class="p-4 gap-4 flex flex-row grow justify-between hover:bg-gray-50 bg-white transition-all items-center rounded-md"
					on:click={() => dispatch('pick', item.title)}
				>
					<div class="flex items-center gap-4">
						<RowIcon kind="script" />

						<div class="w-full text-left font-normal ">
							<div class="text-gray-900 flex-wrap text-md font-semibold mb-1">
								{#if item.marked}
									{@html item.marked ?? ''}
								{:else}
									{item.title ?? ''}
								{/if}
							</div>
						</div>
					</div>
				</button>
			</li>
		{/each}
	</ul>
{/if}
