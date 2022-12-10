<script lang="ts">
	import { hubScripts } from '$lib/stores'
	import { createEventDispatcher, onMount } from 'svelte'
	import type { HubItem } from './model'
	import { Badge, Skeleton } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { capitalize, classNames, loadHubScripts } from '$lib/utils'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import ListFilters from '$lib/components/home/ListFilters.svelte'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'
	export let filter = ''

	const dispatch = createEventDispatcher()

	let filteredItems: (HubItem & { marked?: string })[] = []
	let appFilter: string | undefined = undefined

	$: items = ($hubScripts ?? []).filter((i) => i.kind === kind)
	$: prefilteredItems = appFilter ? (items ?? []).filter((i) => i.app == appFilter) : items ?? []
	$: apps = Array.from(new Set(filteredItems?.map((x) => x.app) ?? [])).sort()

	onMount(() => {
		if (!$hubScripts) {
			loadHubScripts()
		}
	})
</script>

<SearchItems {filter} items={prefilteredItems} bind:filteredItems f={(x) => x.summary} />
<div class="w-full flex mt-1 items-center gap-2">
	<slot />
	<input type="text" placeholder="Search Hub Scripts" bind:value={filter} class="text-2xl grow" />
</div>
<ListFilters filters={apps} bind:selectedFilter={appFilter} resourceType />

{#if $hubScripts}
	{#if filteredItems.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y divide-gray-200 border rounded-md">
			{#each filteredItems as item (item.path)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow hover:bg-gray-50 bg-white transition-all items-center"
						on:click={() => dispatch('pick', item)}
					>
						<div class="flex items-center gap-4">
							<div
								class={classNames(
									'rounded-md p-1 flex justify-center items-center border',
									'bg-gray-50 border-gray-200'
								)}
							>
								<svelte:component
									this={APP_TO_ICON_COMPONENT[item['app']]}
									height={18}
									width={18}
								/>
							</div>

							<div class="w-full text-left font-normal ">
								<div class="text-gray-900 flex-wrap text-md font-semibold mb-1">
									{#if item.marked}
										{@html item.marked ?? ''}
									{:else}
										{item.summary ?? ''}
									{/if}
								</div>
								<div class="text-gray-600 text-xs ">
									{item.path}
								</div>
							</div>
						</div>
						{#if kind !== 'script'}
							<Badge color="gray" baseClass="border">{capitalize(kind)}</Badge>
						{/if}
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
