<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { ScriptService } from '$lib/gen'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { Skeleton } from '$lib/components/common'
	import { emptyString } from '$lib/utils'
	import { Code2 } from 'lucide-svelte'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'
	export let isTemplate: boolean | undefined = undefined

	type Item = {
		path: string
		summary?: string
		description?: string
		hash?: string
	}

	let items: Item[] | undefined = undefined

	let filteredItems: (Item & { marked?: string })[] | undefined = undefined
	export let filter = ''
	export let owners: string[] = []

	$: $workspaceStore && kind && loadItems()

	async function loadItems(): Promise<void> {
		items = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			kinds: kind,
			isTemplate
		})
	}

	export let ownerFilter:
		| { kind: 'inline' | 'owner' | 'integrations'; name: string | undefined }
		| undefined = undefined
	$: if ($workspaceStore) {
		ownerFilter = undefined
	}
	$: prefilteredItems = ownerFilter
		? items?.filter((x) => x.path.startsWith(ownerFilter?.name!))
		: items

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort((a, b) => {
		if (a.startsWith('u/') && !b.startsWith('u/')) return -1
		if (b.startsWith('u/') && !a.startsWith('u/')) return 1

		if (a.startsWith('f/') && !b.startsWith('f/')) return -1
		if (b.startsWith('f/') && !a.startsWith('f/')) return 1

		return a.localeCompare(b)
	})

	const dispatch = createEventDispatcher()
	let lockHash = false
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>

{#if filteredItems}
	{#if filter.length > 0 && filteredItems.length == 0}
		<div class="text-2xs text-tercary text-center py-2 px-3 items-center"> No items found. </div>
	{/if}
	<ul>
		{#each filteredItems as { path, hash, summary, marked }}
			<li class="w-full">
				<button
					class="px-3 py-2 gap-2 flex flex-row w-full hover:bg-surface-hover transition-all items-center rounded-md"
					on:click={() => {
						dispatch('pickScript', { path, hash: lockHash ? hash : undefined })
					}}
				>
					<Code2 size={14} />
					<span class="grow min-w-0 truncate text-left text-2xs text-primary font-normal">
						{#if marked}
							{@html marked}
						{:else}
							{!summary || summary.length == 0 ? path : summary}
						{/if}</span
					>
				</button>
			</li>
		{/each}
	</ul>
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [1.5]]} />
	{/each}
{/if}
