<script lang="ts">
	import { RotateCw } from 'lucide-svelte'
	import { Alert, Button, Drawer, Skeleton } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import NoItemFound from './home/NoItemFound.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import SearchItems from './SearchItems.svelte'
	import { sendUserToast } from '$lib/toast'

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
	let loadError: string | undefined = $state(undefined)
	let items: Item[] | undefined = $state([])
	let filteredItems: Item[] | undefined = $state([])
	let filter = $state('')

	// Only the newest load may write `items`: a slower earlier request can resolve last.
	// Skeletons replace a list that is known-stale; the refresh button omits them so a
	// known-good list does not flicker.
	let loadSeq = 0
	function load(showSkeleton = false): Promise<void> {
		const seq = ++loadSeq
		if (showSkeleton) {
			loading = true
		}
		return loadItems()
			.then((v) => {
				if (seq === loadSeq) {
					items = v
					loadError = undefined
				}
			})
			.catch((err) => {
				if (seq !== loadSeq) return
				// Drop the list rather than keep offering entries the failed load may have
				// superseded. `loadError` then has to carry the reason, or an empty list reads
				// as an empty workspace. An empty body must not win over the message, or the
				// error state is skipped for a falsy `loadError` — hence `||`, not `??`.
				items = []
				loadError = err.body || err.message || String(err)
				// 401/403 are handled globally by onunhandledrejection (logout, privilege
				// toast). No caller awaits load(), so rethrowing still reaches it.
				if (err?.status === 401 || err?.status === 403) {
					throw err
				}
				sendUserToast(`Failed to load ${itemName.toLowerCase()}s: ${loadError}`, true)
			})
			.finally(() => {
				if (seq === loadSeq) {
					loading = false
				}
			})
	}

	export function openDrawer() {
		load(true)
		drawer?.openDrawer?.()
	}

	/** Re-runs `loadItems` against what it closes over now. No-op while closed —
	 * opening reloads anyway. */
	export function reloadItems() {
		if (drawer?.isOpen()) {
			load(true)
		}
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
					on:click={() => {
						refreshing = true
						load().finally(() => {
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
			{:else if loadError}
				<Alert type="error" size="xs" title="Failed to load {itemName.toLowerCase()}s">
					{loadError}
				</Alert>
			{:else if !items?.length}
				<div class="text-center text-sm text-primary mt-2">
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
												variant="default"
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
