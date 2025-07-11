<script lang="ts">
	import { RotateCw } from 'lucide-svelte'
	import { Button, Drawer, Skeleton } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import NoItemFound from './home/NoItemFound.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import SearchItems from './SearchItems.svelte'

	type Item = Record<string, any>

	interface Props {
		pickCallback: (path: string, extraField: string, extraField2: string) => void
		loadItems: () => Promise<Item[] | undefined>
		extraField?: string
		extraField2?: string | undefined
		itemName: string
		closeOnClick?: boolean
		/** Displayed if the load function returns no items. */
		noItemMessage?: string
		/** Displayed if the search returns no items. */
		buttons?: Record<string, (x: string) => void>
		tooltip?: string
		documentationLink?: string | undefined
		submission?: import('svelte').Snippet
	}

	let {
		pickCallback,
		loadItems,
		extraField = 'path',
		extraField2 = undefined,
		itemName,
		closeOnClick = true,
		noItemMessage = 'There are no items in the list',
		buttons = {},
		tooltip = '',
		documentationLink = undefined,
		submission
	}: Props = $props()

	let loading = $state(false)
	let items: Item[] | undefined = $state([])
	let filteredItems: Item[] | undefined = $state([])
	let filter = $state('')

	export function openDrawer() {
		loading = true
		loadItems()
			.then((v) => {
				items = v
			})
			.finally(() => {
				loading = false
			})
		drawer?.openDrawer?.()
	}

	let drawer: Drawer | undefined = $state()

	let refreshing = $state(false)
</script>

<SearchItems
	{filter}
	{items}
	bind:filteredItems
	f={(x) =>
		(extraField2 ? x[extraField2] + ' ' : '') +
		(x[extraField] ?? '') +
		' ' +
		(x['path'] && x['path'] != x[extraField] ? '(' + x['path'] + ')' : '') +
		' ' +
		(x['description'] != x[extraField] ? (x['description'] ?? '') : '')}
/>

<Drawer bind:this={drawer} size="600px">
	<DrawerContent
		{tooltip}
		{documentationLink}
		overflow_y={false}
		title="Search {itemName}s"
		on:close={drawer.closeDrawer}
	>
		<div class="w-full h-full flex flex-col">
			<div class="flex flex-row gap-2 pb-4">
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					placeholder="Search {itemName}s"
					bind:value={filter}
					class="search-item"
					autofocus
				/>
				<Button
					color="light"
					variant="border"
					on:click={() => {
						refreshing = true
						loadItems()
							.then((v) => {
								items = v
							})
							.finally(() => {
								loading = false
								refreshing = false
							})
					}}
					iconOnly
					startIcon={{ icon: RotateCw, classes: loading || refreshing ? 'animate-spin' : '' }}
				/>
			</div>
			{#if loading}
				{#each new Array(3) as _}
					<Skeleton layout={[[5], 0.2]} />
				{/each}
			{:else if !items?.length}
				<div class="text-center text-sm text-tertiary mt-2">
					{@html noItemMessage}
				</div>
			{:else if filteredItems?.length}
				<div class="border rounded-md divide-y w-full overflow-auto pb-12 grow">
					{#each filteredItems as obj}
						<div
							class="hover:bg-surface-hover w-full flex items-center p-4 gap-4 first-of-type:!border-t-0
						first-of-type:rounded-t-md last-of-type:rounded-b-md"
						>
							<div class="inline-flex items-center grow">
								<button
									class="py-2 px-1 gap-1 flex grow border-gray-300 border-opacity-0
									 text-primary"
									onclick={() => {
										if (closeOnClick) {
											drawer?.closeDrawer()
										}
										pickCallback(obj['path'], obj[extraField], extraField2 ? obj[extraField2] : '')
									}}
								>
									{#if `app` in obj}
										<div class="mr-2 text-sm text-left center-center w-30">
											<IconedResourceType after={true} silent={false} name={obj['app']} />
										</div>
									{/if}
									{#if `resource_type` in obj}
										<div class="mr-2 text-left w-30 center-center text-sm">
											<IconedResourceType after={true} name={obj['resource_type']} />
										</div>
									{/if}
									<div class="flex grow flex-col break-all overflow-hidden">
										{#if obj.marked}
											<div class="text-sm font-semibold text-left">
												{@html obj.marked}
											</div>
										{:else}
											<div class="text-sm font-semibold flex flex-col">
												<span class="mr-2 text-left">{obj[extraField] ?? ''}</span>
												{#if extraField != 'path'}
													<span class="font-normal text-xs text-left italic"
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
							</div>
							{#if buttons}
								<div class="flex flex-row items-center">
									{#each Object.entries(buttons) as [name, button]}
										<div>
											<Button
												size="sm"
												variant="border"
												on:click={() => {
													button(obj['path'] ?? '')
												}}
											>
												{name}
											</Button>
										</div>
									{/each}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{:else}
				<NoItemFound />
			{/if}
		</div>
		{#snippet actions()}
			{@render submission?.()}
		{/snippet}
	</DrawerContent>
</Drawer>
