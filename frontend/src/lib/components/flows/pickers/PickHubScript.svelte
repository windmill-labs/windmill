<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Badge, Skeleton } from '$lib/components/common'
	import { capitalize, classNames, sendUserToast } from '$lib/utils'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import { IntegrationService, ScriptService } from '$lib/gen'

	export let kind: 'script' | 'trigger' | 'approval' | 'failure' = 'script'
	export let filter = ''
	export let syncQuery = false

	const dispatch = createEventDispatcher()

	let appFilter: string | undefined = undefined
	let items: {
		path: string
		summary: string
		id: number
		ask_id: number
		app: string
		kind: 'script' | 'trigger' | 'approval' | 'failure' | 'command'
	}[] = []

	let allApps: string[] = []
	let apps: string[] = []

	$: apps = filter.length > 0 ? Array.from(new Set(items?.map((x) => x.app) ?? [])).sort() : allApps

	$: applyFilter(filter, kind, appFilter)
	$: getAllApps(kind)

	async function getAllApps(filterKind: typeof kind) {
		try {
			allApps = (
				await IntegrationService.listHubIntegrations({
					kind: filterKind
				})
			).map((x) => x.name)
			console.log('allApps', allApps)
		} catch (err) {
			sendUserToast(err.message, true)
		}
	}

	let doneTs = 0
	async function applyFilter(
		filter: string,
		filterKind: typeof kind,
		appFilter: string | undefined
	) {
		try {
			const ts = Date.now()
			const scripts =
				filter.length > 0
					? await ScriptService.queryHubScripts({
							text: `${filter}`,
							limit: 40,
							kind: filterKind,
							app: appFilter
					  })
					: (
							await ScriptService.getTopHubScripts({
								limit: 40,
								app: appFilter,
								kind: filterKind
							})
					  ).asks ?? []
			if (ts < doneTs) return
			doneTs = ts
			const processed = scripts.map((x) => ({
				...x,
				path: `hub/${x.version_id}/${x.app}/${x.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
				summary: `${x.summary} (${x.app})`
			}))
			items = processed
		} catch (err) {
			sendUserToast(err.message, true)
		}
	}
</script>

<!-- <SearchItems {filter} items={prefilteredItems} bind:filteredItems f={(x) => x.summary} /> -->
<div class="w-full flex mt-1 items-center gap-2">
	<slot />
	<input type="text" placeholder="Search Hub Scripts" bind:value={filter} class="text-2xl grow" />
</div>

{#if items.length > 0 && apps.length > 0}
	<ListFilters {syncQuery} filters={apps} bind:selectedFilter={appFilter} resourceType />
	{#if items.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md">
			{#each items as item (item.path)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
						on:click={() => dispatch('pick', item)}
					>
						<div class="flex items-center gap-4">
							<div
								class={classNames(
									'rounded-md p-1 flex justify-center items-center border',
									'bg-surface border'
								)}
							>
								<svelte:component
									this={APP_TO_ICON_COMPONENT[item['app']]}
									height={18}
									width={18}
								/>
							</div>

							<div class="w-full text-left font-normal">
								<div class="text-primary flex-wrap text-md font-semibold mb-1">
									{item.summary ?? ''}
								</div>
								<div class="text-secondary text-xs">
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
	{#if items.length == 40}
		<div class="text-tertiary text-sm py-4">
			There are more items than being displayed. Refine your search.
		</div>
	{/if}
{:else}
	<div class="my-2" />
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [4]]} />
	{/each}
{/if}
