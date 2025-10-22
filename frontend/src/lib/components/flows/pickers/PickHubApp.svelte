<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte'
	import { Badge, ButtonType, Skeleton } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { loadHubApps } from '$lib/hub'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		filter?: string
		syncQuery?: boolean
		children?: import('svelte').Snippet
		size?: ButtonType.UnifiedSize
	}

	let { filter = $bindable(''), syncQuery = false, children, size = 'md' }: Props = $props()

	type Item = { apps: string[]; summary: string; path: string }
	let hubApps: any[] | undefined = $state(undefined)
	let filteredItems: (Item & { marked?: string })[] = $state([])
	let appFilter: string | undefined = $state(undefined)

	const prefilteredItems = $derived(
		appFilter ? (hubApps ?? []).filter((i: Item) => i.apps.includes(appFilter!)) : (hubApps ?? [])
	)

	const apps = $derived(Array.from(new Set(filteredItems?.flatMap((x) => x.apps) ?? [])).sort())

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
<div class="w-full flex items-center gap-2">
	{@render children?.()}
	<TextInput
		inputProps={{
			placeholder: 'Search Hub Apps'
		}}
		bind:value={filter}
		class="grow !pr-9"
		{size}
	/>
</div>
<ListFilters {syncQuery} filters={apps} bind:selectedFilter={appFilter} resourceType />

{#if hubApps}
	{#if filteredItems.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md bg-surface-tertiary">
			{#each filteredItems as item (item)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow justify-between hover:bg-surface-hover transition-all items-center"
						onclick={() => dispatch('pick', item)}
					>
						<div class="flex items-center gap-4">
							<RowIcon kind="app" />

							<div class="w-full text-left">
								<div class="text-emphasis flex-wrap text-xs font-semibold">
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
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[[4], 0.5]} />
	{/each}
{/if}
