<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte'
	import { Badge, Skeleton } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { loadHubApps } from '$lib/hub'

	export let filter = ''
	export let syncQuery = false

	type Item = { apps: string[]; summary: string; path: string }
	let hubApps: any[] | undefined = undefined
	let filteredItems: (Item & { marked?: string })[] = []
	let appFilter: string | undefined = undefined

	$: prefilteredItems = appFilter
		? (hubApps ?? []).filter((i) => i.apps.includes(appFilter))
		: hubApps ?? []

	$: apps = Array.from(new Set(filteredItems?.flatMap((x) => x.apps) ?? [])).sort()

	const dispatch = createEventDispatcher()

	onMount(async () => {
		hubApps = await loadHubApps()
	})
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => x.summary + ' (' + x.apps.join(', ') + ')'}
/>
<div class="w-full flex mt-1 items-center gap-2">
	<slot />
	<input type="text" placeholder="Search Hub Apps" bind:value={filter} class="text-2xl grow" />
</div>
<ListFilters {syncQuery} filters={apps} bind:selectedFilter={appFilter} resourceType />

{#if hubApps}
	{#if filteredItems.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md">
			{#each filteredItems as item (item)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow justify-between hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
						on:click={() => dispatch('pick', item)}
					>
						<div class="flex items-center gap-4">
							<RowIcon kind="app" />

							<div class="w-full text-left font-normal">
								<div class="text-primary flex-wrap text-md font-semibold">
									{#if item.marked}
										{@html item.marked ?? ''}
									{:else}
										{item.summary ?? ''}
									{/if}
								</div>
							</div>
						</div>
						<div class="min-w-1/3 gap-2 flex flex-wrap justify-end">
							{#each item.apps as app}
								<Badge color="gray" baseClass="border">{app}</Badge>
							{/each}
						</div>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
{:else}
	<div class="my-2"></div>
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[[4], 0.5]} />
	{/each}
{/if}
