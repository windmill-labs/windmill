<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { ScriptService } from '$lib/gen'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { Badge, Skeleton } from '$lib/components/common'
	import { fade } from 'svelte/transition'
	import { flip } from 'svelte/animate'
	import { emptyString, truncateHash } from '$lib/utils'
	import Toggle from '$lib/components/Toggle.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'
	export let isTemplate: boolean | undefined = undefined
	export let displayLock = false

	type Item = {
		path: string
		summary?: string
		description?: string
		hash?: string
	}

	let items: Item[] | undefined = undefined

	let filteredItems: (Item & { marked?: string })[] | undefined = undefined
	export let filter = ''

	$: $workspaceStore && kind && loadItems()

	async function loadItems(): Promise<void> {
		items = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			kinds: kind,
			isTemplate
		})
	}

	let ownerFilter: string | undefined = undefined
	$: if ($workspaceStore) {
		ownerFilter = undefined
	}
	$: prefilteredItems = ownerFilter ? items?.filter((x) => x.path.startsWith(ownerFilter!)) : items

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	const dispatch = createEventDispatcher()
	let lockHash = false
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>
<div class="flex flex-col min-h-0">
	<div class="w-full flex mt-1 items-center gap-2 mb-3">
		<slot />

		<input
			type="text"
			on:keydown|stopPropagation
			placeholder="Search Workspace Scripts"
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
		{#if displayLock}
			<div class="flex flex-row-reverse mb-1">
				<Toggle
					bind:checked={lockHash}
					options={{ left: 'Latest version', right: 'Lock current hash permanently' }}
				/>
			</div>
		{/if}
		{#if filter.length > 0 && filteredItems.length == 0}
			<NoItemFound />
		{/if}
		<ul class="divide-y border rounded-md">
			{#each filteredItems as { path, hash, summary, description, marked }}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-1 flex flex-row grow hover:bg-surface-hover bg-surface transition-all text-primary"
						on:click={() => {
							dispatch('pick', { path, hash: lockHash ? hash : undefined })
						}}
					>
						<div class="flex flex-col">
							<div class="text-sm font-semibold flex flex-col">
								<span class="mr-2 text-left">
									{#if marked}
										{@html marked}
									{:else}
										{!summary || summary.length == 0 ? path : summary}
									{/if}</span
								>
								<span class="font-normal text-xs text-left italic overflow-hidden"
									>{path ?? ''}</span
								>
							</div>
							<div class="text-xs font-light italic text-left">{description ?? ''}</div>
						</div>
						{#if lockHash}<Badge large baseClass="ml-4">{truncateHash(hash ?? '')}</Badge>{/if}
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
