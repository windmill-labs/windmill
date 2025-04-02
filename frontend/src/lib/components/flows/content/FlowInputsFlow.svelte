<script lang="ts">
	import { Badge, Skeleton } from '$lib/components/common'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString } from '$lib/utils'

	import { createEventDispatcher } from 'svelte'
	import { flip } from 'svelte/animate'
	import { fade } from 'svelte/transition'

	// export let failureModule: boolean
	const dispatch = createEventDispatcher()

	let items: Flow[] | undefined = undefined
	let filteredItems: (Flow & { marked?: string })[] | undefined = undefined
	let filter = ''
	$: $workspaceStore && loadFlows()

	let ownerFilter: string | undefined = undefined
	$: prefilteredItems = ownerFilter ? items?.filter((x) => x.path.startsWith(ownerFilter!)) : items

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	async function loadFlows() {
		items = await FlowService.listFlows({ workspace: $workspaceStore! })
	}
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>
<div class="flex flex-col min-h-0 p-4">
	<h3 class="mb-4">Pick a Workspace Flow</h3>
	<div class="w-full flex mt-1 items-center gap-2 mb-3">
		<slot />

		<input
			type="text"
			placeholder="Search Workspace Flow"
			bind:value={filter}
			class="text-2xl grow"
		/>
	</div>

	{#if filteredItems}
		{#if owners.length > 0}
			<div class="gap-2 w-full flex flex-wrap my-2">
				{#each owners as owner (owner)}
					<div in:fade={{ duration: 50 }} animate:flip={{ duration: 100 }}>
						<Badge
							class="cursor-pointer hover:bg-gray-200"
							on:click={() => {
								ownerFilter = ownerFilter == owner ? undefined : owner
							}}
							color={owner === ownerFilter ? 'blue' : 'gray'}
							baseClass={owner === ownerFilter ? 'border border-blue-500' : 'border'}
						>
							{owner}
							{#if owner === ownerFilter}&cross;{/if}
						</Badge>
					</div>
				{/each}
			</div>
		{/if}
		{#if filter.length > 0 && filteredItems.length == 0}
			<NoItemFound />
		{/if}
		<ul class="divide-y overflow-auto">
			{#each filteredItems as { path, summary, description, marked }}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-1 flex flex-row grow hover:bg-surface-hover bg-surface transition-all text-primary"
						on:click={() => {
							dispatch('pick', { path })
						}}
					>
						<div class="flex flex-col">
							<div class="text-sm font-semibold flex flex-col">
								<span class="mr-2 text-left">
									{#if marked}
										{@html marked}
									{:else}
										{!summary || summary.length == 0 ? path : summary}
									{/if}
								</span>
								<span class="font-normal text-xs text-left italic overflow-hidden"
									>{path ?? ''}
								</span>
							</div>
							<div class="text-xs font-light italic text-left">{description ?? ''}</div>
						</div>
					</button>
				</li>
			{/each}
		</ul>
	{:else}
		<div class="mt-6"></div>

		{#each new Array(6) as _}
			<Skeleton layout={[[4], 0.7]} />
		{/each}
	{/if}
</div>
