<!-- PickHubItems.svelte -->
<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte'
	import { Skeleton, ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { loadHubScripts, loadHubFlows, loadHubApps } from '$lib/utils'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import { hubScripts, hubFlows, hubApps, type HubItem } from '$lib/stores'
	import PickHubScript from './PickHubScript.svelte'
	import PickHubFlow from './PickHubFlow.svelte'
	import PickHubApp from './PickHubApp.svelte'
	import { faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import { Code2, LayoutDashboard } from 'lucide-svelte'
	import { Icon } from 'svelte-awesome'

	export let filter = ''
	export let selectedKind: 'script' | 'flow' | 'app' | undefined = undefined
	export let kind: 'script' | 'trigger' | 'approval' | 'failure' | undefined = undefined

	let itemKind: 'script' | 'flow' | 'app' = 'app'
	const dispatch = createEventDispatcher()
	let filteredItems: (HubItem & { marked?: string })[] = []
	let appFilter: string | undefined = undefined

	$: items = [
		...($hubScripts?.map((s) => ({ itemType: 'script', data: s })) ?? []),
		...($hubFlows?.map((s) => ({ itemType: 'flow', data: s })) ?? []),
		...($hubApps?.map((s) => ({ itemType: 'app', data: s })) ?? [])
	] as HubItem[]

	$: apps = Array.from(
		new Set(
			filteredItems
				?.map((x) => {
					if (x.itemType === 'script') {
						return x.data.app
					} else if (x.itemType === 'flow') {
						return x.data.apps
					} else if (x.itemType === 'app') {
						return x.data.apps
					} else {
						return undefined
					}
				})
				.filter(Boolean)
				.flat() ?? []
		)
	).sort() as string[]

	$: prefilteredItems =
		appFilter || itemKind
			? items.filter((i) => {
					if (i.itemType !== itemKind) {
						return false
					}

					if (selectedKind && i.itemType !== selectedKind) {
						return false
					}

					if (i.itemType === 'script' && appFilter) {
						return i.data.app === appFilter
					} else if (i.itemType === 'flow' && appFilter) {
						return i.data.apps.includes(appFilter)
					} else if (i.itemType === 'app' && appFilter) {
						return i.data.apps.includes(appFilter)
					}
					return true
			  })
			: items

	onMount(async () => {
		await loadHubScripts()
		$hubFlows = await loadHubFlows()
		$hubApps = await loadHubApps()
	})
</script>

<SearchItems {filter} items={prefilteredItems} bind:filteredItems f={(x) => x.data.summary} />
<div class="w-full flex mt-1 items-center gap-2">
	{#if selectedKind === undefined}
		<ToggleButtonGroup bind:selected={itemKind}>
			<ToggleButton light position="left" value="script" size="sm">
				<div class="flex gap-1 items-center">
					<Code2 size={16} />
					Scripts
				</div>
			</ToggleButton>
			<ToggleButton light position="center" value="flow" size="sm">
				<div class="flex gap-1 items-center">
					<Icon data={faBarsStaggered} scale={0.8} class="mr-1" />
					Flows
				</div>
			</ToggleButton>
			<ToggleButton light position="right" value="app" size="sm">
				<div class="flex gap-1 items-center">
					<LayoutDashboard size={16} />
					Apps
				</div>
			</ToggleButton>
		</ToggleButtonGroup>
	{/if}
	<input type="text" placeholder="Search Hub Items" bind:value={filter} class="text-2xl grow" />
	<slot />
</div>

<ListFilters filters={apps} bind:selectedFilter={appFilter} resourceType />

{#if Array.isArray(prefilteredItems)}
	{#if filteredItems.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y divide-gray-200 border rounded-md">
			{#each filteredItems as item, index (index)}
				<li class="flex flex-row w-full">
					{#if item.itemType === 'script'}
						<PickHubScript
							item={item.data}
							on:pick={() => dispatch('pickScript', item.data)}
							{kind}
						/>
					{:else if item.itemType === 'flow'}
						<PickHubFlow item={item.data} on:pick={() => dispatch('pickFlow', item.data)} />
					{:else if item.itemType === 'app'}
						<PickHubApp item={item.data} on:pick={() => dispatch('pickApp', item.data)} />
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
{:else}
	<div class="my-2" />
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[[4], 0.5]} />
	{/each}
{/if}
