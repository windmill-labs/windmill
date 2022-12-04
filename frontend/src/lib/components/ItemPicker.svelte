<script lang="ts">
	import { Drawer, Skeleton } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import SearchItems from './SearchItems.svelte'

	type Item = Record<string, any>
	export let pickCallback: (path: string, f: string) => void
	export let loadItems: () => Promise<Item[] | undefined>
	export let extraField: string = 'path'
	export let extraField2: string | undefined
	export let itemName: string
	export let closeOnClick = true
	/** Displayed if the load function returns no items. */
	export let noItemMessage = 'There are no items in the list'
	/** Displayed if the search returns no items. */
	export let emptySearchMessage = 'No items found'
	export let buttons: Record<string, (x: string) => void> = {}

	let loading = false
	let items: Item[] | undefined = []
	let filteredItems: Item[] | undefined = []
	let filter = ''

	export function openDrawer() {
		loading = true
		loadItems().then((v) => {
			items = v
			loading = false
		})
		drawer.openDrawer?.()
	}

	let drawer: Drawer
</script>

<SearchItems
	{filter}
	{items}
	bind:filteredItems
	f={(x) =>
		(extraField2 ? x[extraField2] + ' ' : '') +
		(x[extraField] ?? '') +
		' ' +
		(x['path'] && x['path'] != x[extraField] ? '(' + x['path'] + ')' ?? '' : '') +
		' ' +
		(x['description'] != x[extraField] ? x['description'] ?? '' : '')}
/>

<Drawer bind:this={drawer} size="600px">
	<DrawerContent title="Search a {itemName}" on:close={drawer.closeDrawer}>
		<div>
			<div class="w-12/12 pb-4">
				<input
					type="text"
					placeholder="Search {itemName}"
					bind:value={filter}
					class="search-item"
				/>
			</div>
			{#if loading}
				{#each new Array(6) as _}
					<Skeleton layout={[[2], 0.7]} />
				{/each}
			{:else if !items?.length}
				<div class="text-center text-sm text-gray-600 mt-2">
					{@html noItemMessage}
				</div>
			{:else if filteredItems?.length}
				<ul class="divide-y divide-gray-200">
					{#each filteredItems as obj}
						<li class="flex flex-row w-full">
							<button
								class="py-4 px-1 gap-1 flex flex-row grow border  border-gray-300 border-opacity-0 
									hover:bg-white hover:border-opacity-100 text-black"
								on:click={() => {
									if (closeOnClick) {
										drawer.closeDrawer()
									}
									pickCallback(obj['path'], obj[extraField])
								}}
							>
								{#if `app` in obj}
									<div class="mr-2 text-sm text-left truncate w-24">
										<IconedResourceType after={true} silent={false} name={obj['app']} />
									</div>
								{/if}
								{#if `resource_type` in obj}
									<div class="mr-2 truncate text-left w-24 text-sm">
										<IconedResourceType after={true} name={obj['resource_type']} />
									</div>
								{/if}
								<div class="flex flex-col">
									{#if obj.marked}
										<div class="text-sm font-semibold">
											{@html obj.marked}
										</div>
									{:else}
										<div class="text-sm font-semibold flex flex-col">
											<span class="mr-2 text-left">{obj[extraField] ?? ''}</span>
											{#if extraField != 'path'}
												<span class="font-normal text-xs text-left italic overflow-hidden"
													>{obj['path'] ?? ''}</span
												>
											{/if}
										</div>
										{#if extraField != 'description'}
											<div class="text-xs font-light italic text-left"
												>{obj['description'] ?? ''}</div
											>
										{/if}
									{/if}
								</div>
							</button>
							{#if buttons}
								<div class="flex flex-row">
									{#each Object.entries(buttons) as [name, button]}
										<button
											class="py-4 px-1 gap-1 flex flex-row grow  text-black"
											on:click={() => {
												button(obj['path'] ?? '')
											}}
										>
											{name}
										</button>
									{/each}
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<div class="text-center text-sm text-gray-600 mt-2">
					{@html emptySearchMessage}
				</div>
			{/if}
		</div>
		<span slot="submission" class="mr-2">
			<slot name="submission" />
		</span>
	</DrawerContent>
</Drawer>
