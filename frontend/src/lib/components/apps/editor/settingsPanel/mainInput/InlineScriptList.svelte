<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher } from 'svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'

	interface Props {
		filter?: string
		inlineScripts?: string[]
		children?: import('svelte').Snippet
	}

	let { filter = $bindable(''), inlineScripts = [], children }: Props = $props()

	type Item = { title: string }
	let filteredItems: (Item & { marked?: string })[] = $state([])
	let items = $derived(inlineScripts.map((x) => ({ title: x })))
	let prefilteredItems = $derived(items ?? [])

	const dispatch = createEventDispatcher()
</script>

<SearchItems {filter} items={prefilteredItems} bind:filteredItems f={(x) => x.summary} />
<div class="w-full flex mt-1 items-center gap-2">
	{@render children?.()}
	<input
		onkeydown={stopPropagation(bubble('keydown'))}
		type="text"
		placeholder="Search inline scripts"
		bind:value={filter}
		class="text-2xl grow mb-4"
	/>
</div>

{#if inlineScripts.length === 0}
	<div class="flex flex-col w-full h-full">
		<div class="text-md">No detached inline scripts</div>
	</div>
{:else if filteredItems.length === 0}
	<NoItemFound />
{:else}
	<ul class="divide-y border rounded-md">
		{#each filteredItems as item (item)}
			<li class="flex flex-row w-full">
				<button
					class="p-4 gap-4 flex flex-row grow justify-between hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
					onclick={() => dispatch('pick', item.title)}
				>
					<div class="flex items-center gap-4">
						<RowIcon kind="script" />

						<div class="w-full text-left font-normal">
							<div class="text-primary flex-wrap text-md font-semibold mb-1">
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
