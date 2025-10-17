<script module lang="ts">
	let listHubIntegrationsCached = createCache(
		({ kind }: { kind: HubScriptKind & string; refreshCount?: number }) =>
			IntegrationService.listHubIntegrations({ kind }),
		{ initial: { kind: 'script', refreshCount: 0 }, invalidateMs: 1000 * 60 }
	)

	let listHubScriptsCached = createCache(
		async ({
			filter,
			kind,
			appFilter
		}: {
			filter: string
			kind: HubScriptKind & string
			appFilter: string | undefined
			refreshCount?: number
		}) => {
			try {
				return get(userStore)
					? filter.length > 0
						? await ScriptService.queryHubScripts({ text: filter, limit: 40, kind })
						: ((await ScriptService.getTopHubScripts({ limit: 40, kind, app: appFilter })).asks ??
							[])
					: undefined
			} catch (err) {
				console.error('Error fetching top hub scripts')
				return undefined
			}
		},
		{
			initial: { filter: '', kind: 'script', appFilter: undefined, refreshCount: 0 },
			invalidateMs: 1000 * 60
		}
	)
</script>

<script lang="ts">
	import { createEventDispatcher, untrack } from 'svelte'
	import { Skeleton } from '$lib/components/common'
	import { classNames, createCache } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import { IntegrationService, ScriptService, type HubScriptKind } from '$lib/gen'
	import { Circle } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { userStore } from '$lib/stores'
	import { get } from 'svelte/store'

	let hubNotAvailable = $state(false)

	const dispatch = createEventDispatcher()

	interface Props {
		kind?: HubScriptKind & string
		filter?: string
		loading?: boolean
		selected?: number | undefined
		appFilter?: string | undefined
		items?: {
			path: string
			summary: string
			id: number
			version_id: number
			ask_id: number
			app: string
			kind: HubScriptKind
		}[]
		displayPath?: boolean
		apps?: string[]
		refreshCount?: number
	}

	let {
		kind = 'script',
		filter = $bindable(''),
		loading = $bindable(false),
		selected = undefined,
		appFilter = undefined,
		items = $bindable([]),
		displayPath = false,
		apps = $bindable([]),
		refreshCount = 0
	}: Props = $props()

	let allApps: string[] = $state([])
	$effect(() => {
		if (filter.length > 0) {
			apps = Array.from(new Set(items?.map((x) => x.app) ?? [])).sort()
		} else {
			apps = allApps
		}
	})

	async function getAllApps(filterKind: typeof kind) {
		try {
			hubNotAvailable = false
			allApps = (await listHubIntegrationsCached({ kind: filterKind, refreshCount })).map(
				(x) => x.name
			)
		} catch (err) {
			console.error('Hub is not available')
			allApps = []
			hubNotAvailable = true
		}
	}

	let hubScriptsFilteredPromise = usePromise(
		() => listHubScriptsCached({ appFilter, filter, kind, refreshCount }),
		{ loadInit: false }
	)
	$effect(() => {
		;[filter, kind, appFilter, refreshCount]
		hubScriptsFilteredPromise.refresh()
	})
	$effect(() => {
		loading = hubScriptsFilteredPromise.status === 'loading'
		hubNotAvailable = !!hubScriptsFilteredPromise.error
		const scripts = hubScriptsFilteredPromise.value
		untrack(() => {
			if (!scripts) return
			const mappedItems = scripts.map(
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

			items = appFilter ? mappedItems.filter((x) => x.app === appFilter) : mappedItems
		})
	})

	function onKeyDown(e: KeyboardEvent) {
		if (
			selected != undefined &&
			items &&
			selected >= 0 &&
			selected < items?.length! &&
			e.key === 'Enter'
		) {
			e.preventDefault()
			let item = items![selected]
			dispatch('pickScript', item)
		}
	}
	$effect(() => {
		;[kind, refreshCount]
		untrack(() => {
			getAllApps(kind)
		})
	})
</script>

<svelte:window onkeydown={onKeyDown} />
{#if hubNotAvailable}
	<div class="text-2xs text-red-400 ftext-2xs font-light text-center py-2 px-3 items-center">
		Hub not available
	</div>
{:else if loading}
	{#each Array(15).fill(0) as _}
		<Skeleton layout={[0.1, [1.5]]} />
	{/each}
{:else if items.length > 0 && apps.length > 0}
	<ul>
		{#each items as item, index (item.path)}
			<li class="w-full">
				<Popover class="w-full" placement="right" forceOpen={index === selected}>
					{#snippet text()}
						<div class="flex flex-col">
							<div class="text-left text-xs font-normal leading-tight py-0"
								>{item.summary ?? ''}</div
							>
							<div class="text-left text-2xs font-normal">
								{item.path ?? ''}
							</div>
						</div>
					{/snippet}
					<button
						class="px-3 py-2 gap-2 flex flex-row w-full hover:bg-surface-hover transition-all items-center rounded-md {index ===
						selected
							? 'bg-surface-hover'
							: ''}"
						onclick={() => dispatch('pickScript', item)}
					>
						<div class={classNames('flex justify-center items-center')}>
							{#if item['app'] in APP_TO_ICON_COMPONENT}
								{@const SvelteComponent = APP_TO_ICON_COMPONENT[item['app']]}
								<SvelteComponent height={14} width={14} />
							{:else}
								<div
									class="w-[14px] h-[14px] text-gray-400 flex flex-row items-center justify-center"
								>
									<Circle size="12" />
								</div>
							{/if}
						</div>

						<div class="flex flex-col grow min-w-0">
							<div
								class="grow truncate text-left text-2xs text-primary font-normal leading-tight py-0.5"
								>{item.summary ?? ''}</div
							>
							{#if displayPath && item.path}
								<div class="grow truncate text-left text-2xs text-secondary font-[220]">
									{item.path}
								</div>
							{/if}
						</div>
						{#if index === selected}
							<kbd class="!text-xs">&crarr;</kbd>
						{/if}
					</button>
				</Popover>
			</li>
		{/each}
	</ul>
	{#if items.length == 40}
		<div class="text-2xs text-tercary font-extralight text-center py-2 px-3 items-center">
			There are more items than being displayed. Refine your search.
		</div>
	{/if}
{/if}
