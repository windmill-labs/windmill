<script lang="ts">
	import { hubScripts } from '$lib/stores'
	import { createEventDispatcher, onMount } from 'svelte'
	import type { HubItem } from './model'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import { Badge, Skeleton } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { loadHubScripts } from '$lib/utils'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'

	$: items = ($hubScripts ?? []).filter((i) => i.kind === kind)

	let filteredItems: (HubItem & { marked?: string })[] = []
	let filter = ''
	let appFilter: string | undefined = undefined

	$: prefilteredItems = appFilter ? (items ?? []).filter((i) => i.app == appFilter) : items ?? []

	$: apps = Array.from(new Set(filteredItems?.map((x) => x.app) ?? [])).sort()

	const dispatch = createEventDispatcher()

	onMount(() => {
		if (!$hubScripts) {
			loadHubScripts()
		}
	})
</script>

<SearchItems {filter} items={prefilteredItems} bind:filteredItems f={(x) => x.summary} />

<div class="flex flex-col min-h-0">
	<div class="w-12/12 pb-2 flex flex-row my-1 gap-1">
		<input type="text" placeholder="Search Hub Scripts" bind:value={filter} class="text-2xl grow" />
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
			{#if $hubScripts}
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
							<div class="mr-2 text-sm text-left truncate w-32 shrink-0">
								<IconedResourceType after={true} silent={false} name={obj['app']} />
							</div>
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
