<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Alert, Skeleton } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import { IntegrationService, ScriptService, type HubScriptKind } from '$lib/gen'

	export let kind: HubScriptKind & string = 'script'
	export let filter = ''

	export let loading = false
	let hubNotAvailable = false

	const dispatch = createEventDispatcher()

	export let appFilter: string | undefined = undefined
	let items: {
		path: string
		summary: string
		id: number
		version_id: number
		ask_id: number
		app: string
		kind: HubScriptKind
	}[] = []

	let allApps: string[] = []
	export let apps: string[] = []

	$: apps = filter.length > 0 ? Array.from(new Set(items?.map((x) => x.app) ?? [])).sort() : allApps

	$: applyFilter(filter, kind, appFilter)
	$: getAllApps(kind)

	async function getAllApps(filterKind: typeof kind) {
		try {
			hubNotAvailable = false
			allApps = (
				await IntegrationService.listHubIntegrations({
					kind: filterKind
				})
			).map((x) => x.name)
		} catch (err) {
			console.error('Hub is not available')
			allApps = []
			hubNotAvailable = true
		}
	}

	let startTs = 0
	async function applyFilter(
		filter: string,
		filterKind: typeof kind,
		appFilter: string | undefined
	) {
		try {
			loading = true
			hubNotAvailable = false
			const ts = Date.now()
			startTs = ts
			await new Promise((r) => setTimeout(r, 100))
			if (ts < startTs) return
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
			if (ts === startTs) {
				loading = false
			}
			items = scripts.map(
				(x: {
					summary: string
					version_id: number
					id: number
					ask_id: number
					app: string
					kind: HubScriptKind
				}) => ({
					...x,
					path: `hub/${x.version_id}/${x.app}/${x.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
					summary: `${x.summary} (${x.app})`
				})
			)
			hubNotAvailable = false
		} catch (err) {
			hubNotAvailable = true
			console.error('Hub not available')
			loading = false
		}
	}
</script>

{#if hubNotAvailable}
	<div class="mt-2" />
	<Alert type="error" title="Hub not available" />
{:else if items.length > 0 && apps.length > 0}
	{#if items.length == 0}
		<NoItemFound />
	{:else}
		<ul>
			{#each items as item (item.path)}
				<li class="w-full">
					<button
						class="px-3 py-2 gap-2 flex flex-row w-full hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
						on:click={() => dispatch('pickScript', item)}
					>
						<div class={classNames('flex justify-center items-center')}>
							{#if item['app'] in APP_TO_ICON_COMPONENT}
								<svelte:component
									this={APP_TO_ICON_COMPONENT[item['app']]}
									height={14}
									width={14}
								/>
							{/if}
						</div>

						<span class="grow truncate text-left text-xs text-primary font-semibold">
							{item.summary ?? ''}
						</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
	{#if items.length == 40}
		<div class="text-2xs text-secondary text-center py-2 px-3 items-center">
			There are more items than being displayed. Refine your search.
		</div>
	{/if}
{:else}
	<div class="my-2" />
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [4]]} />
	{/each}
{/if}
