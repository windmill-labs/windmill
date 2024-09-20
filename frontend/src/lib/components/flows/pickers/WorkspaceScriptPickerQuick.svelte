<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { ScriptService } from '$lib/gen'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { Badge, Skeleton } from '$lib/components/common'
	import { emptyString, truncateHash } from '$lib/utils'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'

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
	{#if filteredItems}
		{#if filter.length > 0 && filteredItems.length == 0}
			<NoItemFound />
		{/if}
		<ul class="border rounded-md">
			{#each filteredItems as { path, hash, summary, marked }}
				<li class="flex flex-row w-full">
					<button
						class="p-1 gap-1 flex flex-row grow hover:bg-surface-hover bg-surface transition-all items-center"
						on:click={() => {
							dispatch('pickScript', { path, hash: lockHash ? hash : undefined })
						}}
					>
						<div class="flex items-center gap-4">
							<span
								class="w-full max-w-60 text-left text-xs text-primary font-semibold mb-1 truncate"
							>
								{#if marked}
									{@html marked}
								{:else}
									{!summary || summary.length == 0 ? path : summary}
								{/if}</span
							>
						</div>

						{#if lockHash}<Badge large baseClass="ml-4">{truncateHash(hash ?? '')}</Badge>{/if}
					</button>
				</li>
			{/each}
		</ul>
	{:else}
		<div class="mt-6" />

		{#each new Array(6) as _}
			<Skeleton layout={[[4], 0.7]} />
		{/each}
	{/if}
</div>
