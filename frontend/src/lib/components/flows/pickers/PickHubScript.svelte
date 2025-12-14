<script lang="ts">
	import { createEventDispatcher, untrack } from 'svelte'
	import { Alert, Badge, ButtonType, Skeleton } from '$lib/components/common'
	import { capitalize } from '$lib/utils'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import { IntegrationService, ScriptService, type HubScriptKind } from '$lib/gen'
	import { Loader2 } from 'lucide-svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		kind?: HubScriptKind & string
		filter?: string
		syncQuery?: boolean
		children?: import('svelte').Snippet
		size?: ButtonType.UnifiedSize
	}

	let {
		kind = 'script',
		filter = $bindable(''),
		syncQuery = false,
		children,
		size = 'md'
	}: Props = $props()

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
							limit: 20,
							kind: filterKind,
							app: appFilter
						})
					: ((
							await ScriptService.getTopHubScripts({
								limit: 20,
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

	async function handlePick(item: (typeof items)[number]) {
		if (item.path.startsWith('hub/')) {
			try {
				await ScriptService.pickHubScriptByPath({ path: item.path })
			} catch (error) {
				console.error('Failed to track hub script pick:', error)
				// Don't block the flow if tracking fails
			}
		}

		// Dispatch the event to continue with the selection
		dispatch('pick', item)
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

<div class="w-full flex items-center gap-2">
	{@render children?.()}
	<div class="relative w-full">
		<TextInput
			inputProps={{
				placeholder: 'Search Hub Scripts'
			}}
			bind:value={filter}
			class="grow !pr-9"
			{size}
		/>
		{#if loading}
			<Loader2 class="animate-spin text-gray-400 absolute right-2 top-1" />
		{/if}
	</div>
</div>

{#if hubNotAvailable}
	<Alert type="error" title="Hub not available" />
{:else if (items.length > 0 && apps.length > 0) || !loading}
	<ListFilters {syncQuery} filters={apps} bind:selectedFilter={appFilter} resourceType />
	{#if items.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md bg-surface-tertiary">
			{#each items as item (item.path)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow hover:bg-surface-hover transition-all items-center"
						onclick={() => handlePick(item)}
					>
						<div class="flex items-center gap-4">
							<div class="flex justify-center items-center">
								{#if item['app'] in APP_TO_ICON_COMPONENT}
									{@const SvelteComponent = APP_TO_ICON_COMPONENT[item['app']]}
									<SvelteComponent height={18} width={18} />
								{/if}
							</div>

							<div class="w-full text-left">
								<div class="text-emphasis flex-wrap text-xs font-semibold mb-1">
									{item.summary ?? ''}
								</div>
								<div class="text-secondary text-2xs font-normal">
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
	{#if items.length == 20}
		<div class="text-primary text-xs font-normal py-4">
			There are more items than being displayed. Refine your search.
		</div>
	{/if}
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [4]]} />
	{/each}
{/if}
