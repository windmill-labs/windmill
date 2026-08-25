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
				console.error('Failed to fetch hub scripts:', err)
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
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import { Skeleton } from '$lib/components/common'
	import { classNames, createCache } from '$lib/utils'
	import { APP_TO_ICON_COMPONENT } from '$lib/components/icons'
	import { IntegrationService, ScriptService, type HubScriptKind } from '$lib/gen'
	import { Circle, ExternalLink } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { disableHubStore, hubBaseUrlStore, userStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Alert } from '$lib/components/common'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import { logHubScriptPick } from '$lib/utils/featureUsage'

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')

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
			/** The hub's own wording, before `summary` is rewritten as the display label. */
			hubSummary: string
			id: number
			version_id: number
			ask_id: number
			app: string
			kind: HubScriptKind
		}[]
		displayPath?: boolean
		apps?: string[]
		refreshCount?: number
		onHover?: (index: number) => void
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
		refreshCount = 0,
		onHover = undefined
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
		if ($disableHubStore) return
		try {
			hubNotAvailable = false
			allApps = (await listHubIntegrationsCached({ kind: filterKind, refreshCount })).map(
				(x) => x.name
			)
		} catch (err) {
			console.error('Failed to fetch hub integrations:', err)
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
		if (!$disableHubStore) {
			hubScriptsFilteredPromise.refresh()
		}
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
					// `summary` below becomes the display label; keep the hub's own wording,
					// which is what telemetry keys off.
					hubSummary: x.summary,
					summary: `${x.summary} (${x.app})`
				})
			)

			items = appFilter ? mappedItems.filter((x) => x.app === appFilter) : mappedItems
		})
	})

	async function handlePickScript(item: (typeof items)[number]) {
		if (item.path.startsWith('hub/')) {
			logHubScriptPick(
				{ version_id: item.version_id, app: item.app, summary: item.hubSummary },
				'picker'
			)
			try {
				await ScriptService.pickHubScriptByPath({ path: item.path })
			} catch (error) {
				console.error('Failed to track hub script pick:', error)
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
{#if $disableHubStore}
	<!-- Hub disabled, show nothing -->
{:else if hubNotAvailable}
	<div class="px-3 py-2 mt-2">
		<Alert type="warning" title="Hub not available" size="xs">
			Could not connect to the Windmill Hub. If you are in a closed environment, you can disable the
			Hub in the <a href="/#superadmin-settings?tab=private_hub">instance settings</a>.
		</Alert>
	</div>
{:else if loading}
	{#each Array(15).fill(0) as _}
		<Skeleton layout={[0.1, [1.5]]} />
	{/each}
{:else if items.length > 0 && apps.length > 0}
	<ul class="gap-1 flex flex-col">
		{#each items as item, index (item.path)}
			<li class="w-full">
				<!-- Only the selected row may show a tooltip: the Popover opens on its own hover too, and a
				     row scrolled under a stationary cursor would otherwise open a second one. -->
				<Popover
					class="w-full"
					placement="right"
					forceOpen={index === selected}
					disablePopup={index !== selected}
				>
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
						variant="subtle"
						unifiedSize="sm"
						btnClasses="justify-start h-auto min-h-7 py-1 {selected === index
							? 'bg-surface-hover'
							: onHover
								? 'hover:bg-transparent'
								: ''}"
						onmousemove={() => onHover?.(index)}
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
							<div class="min-w-0 truncate text-left font-normal leading-tight"
								>{item.summary ?? ''}</div
							>
							{#if displayPath && item.path}
								<div class="min-w-0 truncate text-left text-2xs font-thin leading-tight">
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
	{:else if customUi?.suggestScript != false}
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
