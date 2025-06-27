<script lang="ts">
	import { createEventDispatcher, untrack } from 'svelte'
	import { Alert, Badge, Skeleton } from '$lib/components/common'
	import { capitalize, classNames } from '$lib/utils'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import { IntegrationService, ScriptService, type HubScriptKind } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'

	interface Props {
		kind?: HubScriptKind & string
		filter?: string
		syncQuery?: boolean
		children?: import('svelte').Snippet
	}

	let { kind = 'script', filter = $bindable(''), syncQuery = false, children }: Props = $props()

	let loading = $state(false)
	let hubNotAvailable = $state(false)

	const dispatch = createEventDispatcher()

	let appFilter: string | undefined = $state(undefined)
	let items: {
		path: string
		summary: string
		id: number
		version_id: number
		ask_id: number
		app: string
		kind: HubScriptKind
	}[] = $state([])

	let allApps: string[] = $state([])
	let apps: string[] = $derived.by(() =>
		filter.length > 0 ? Array.from(new Set(items?.map((x) => x.app) ?? [])).sort() : allApps
	)

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
					: ((
							await ScriptService.getTopHubScripts({
								limit: 40,
								app: appFilter,
								kind: filterKind
							})
						).asks ?? [])
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

	$effect(() => {
		;[filter, kind, appFilter]
		untrack(() => applyFilter(filter, kind, appFilter))
	})
	$effect(() => {
		kind
		untrack(() => getAllApps(kind))
	})
</script>

<div class="w-full flex mt-1 items-center gap-2">
	{@render children?.()}
	<div class="relative w-full">
		<input
			type="text"
			placeholder="Search Hub Scripts"
			bind:value={filter}
			class="text-2xl grow !pr-9"
		/>
		{#if loading}
			<Loader2 class="animate-spin text-gray-400 absolute right-2 top-2.5" />
		{/if}
	</div>
</div>

{#if hubNotAvailable}
	<div class="mt-2"></div>
	<Alert type="error" title="Hub not available" />
{:else if items.length > 0 && apps.length > 0}
	<ListFilters {syncQuery} filters={apps} bind:selectedFilter={appFilter} resourceType />
	{#if items.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md">
			{#each items as item (item.path)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
						onclick={() => dispatch('pick', item)}
					>
						<div class="flex items-center gap-4">
							<div
								class={classNames(
									'rounded-md p-1 flex justify-center items-center border',
									'bg-surface border'
								)}
							>
								{#if item['app'] in APP_TO_ICON_COMPONENT}
									{@const SvelteComponent = APP_TO_ICON_COMPONENT[item['app']]}
									<SvelteComponent height={18} width={18} />
								{/if}
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
	<div class="my-2"></div>
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [4]]} />
	{/each}
{/if}
