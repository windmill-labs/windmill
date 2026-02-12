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
						? await ScriptService.queryHubScripts({ text: filter, limit: 20, kind })
						: ((await ScriptService.getTopHubScripts({ limit: 20, kind, app: appFilter })).asks ??
							[])
					: undefined
			} catch (err) {
				sendUserToast('Failed to fetch hub scripts: ' + err, 'error')
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
	import { classNames, createCache, sendUserToast } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import { IntegrationService, ScriptService, type HubScriptKind } from '$lib/gen'
	import { Circle, ExternalLink } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { hubBaseUrlStore, userStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import Button from '$lib/components/common/button/Button.svelte'

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
			sendUserToast('Failed to fetch hub integrations: ' + err, 'error')
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

	async function handlePickScript(item: (typeof items)[number]) {
		if (item.path.startsWith('hub/')) {
			try {
				await ScriptService.pickHubScriptByPath({ path: item.path })
			} catch (error) {
				sendUserToast('Failed to call ScriptService.pickHubScriptByPath: ' + error, 'error')
				// Don't block the flow if tracking fails
			}
		}

		// Dispatch the event to continue with the selection
		dispatch('pickScript', item)
	}

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
			handlePickScript(item)
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
	<div class="text-2xs text-red-400 font-normal text-center py-2 px-3 items-center">
		Hub not available
	</div>
{:else if loading}
	{#each Array(15).fill(0) as _}
		<Skeleton layout={[0.1, [1.5]]} />
	{/each}
{:else if items.length > 0 && apps.length > 0}
	<ul class="gap-1 flex flex-col">
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
					<Button
						selected={selected === index}
						variant="subtle"
						unifiedSize="sm"
						btnClasses="justify-start"
						onClick={() => handlePickScript(item)}
					>
						<div class={classNames('flex justify-center items-center')}>
							{#if item['app'] in APP_TO_ICON_COMPONENT}
								{@const SvelteComponent = APP_TO_ICON_COMPONENT[item['app']]}
								<SvelteComponent height={13} width={13} />
							{:else}
								<div class="text-gray-400 flex flex-row items-center justify-center">
									<Circle size="13" />
								</div>
							{/if}
						</div>

						<div class="flex flex-col grow min-w-0">
							<div class="grow truncate text-left font-normal leading-tight py-0.5"
								>{item.summary ?? ''}</div
							>
							{#if displayPath && item.path}
								<div class="grow truncate text-left text-2xs font-thin">
									{item.path}
								</div>
							{/if}
						</div>
						{#if index === selected}
							<kbd class="!text-xs">&crarr;</kbd>
						{/if}
					</Button>
				</Popover>
			</li>
		{/each}
	</ul>
	{#if items.length == 20}
		<div class="text-2xs text-tercary font-extralight text-center py-2 px-3 items-center">
			There are more items than being displayed. Refine your search.
		</div>
	{:else}
		<div class="px-2 py-1">
			<a
				href={`${$hubBaseUrlStore}?suggest_script=true`}
				target="_blank"
				class="text-xs flex flex-row items-center gap-1 text-blue-500 hover:text-blue-600"
				>Suggest script <ExternalLink class="size-3" />
			</a>
		</div>
	{/if}
{:else}
	<div class="text-2xs text-primary font-light text-center py-2 px-3 items-center">
		No scripts found.
	</div>
{/if}
