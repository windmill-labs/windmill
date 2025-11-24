<script module lang="ts">
	let initialWorkspace = get(workspaceStore)
	let loadItemsCached = createCache(
		({
			workspace,
			kind,
			isTemplate
		}: {
			workspace?: string
			kind?: string
			isTemplate?: boolean
			refreshCount?: number
		}) =>
			workspace && get(userStore)
				? kind == 'flow'
					? FlowService.listFlows({ workspace, withoutDescription: true })
					: ScriptService.listScripts({
							workspace,
							kinds: kind,
							isTemplate,
							withoutDescription: true
						})
				: undefined,
		initialWorkspace
			? {
					initial: {
						workspace: initialWorkspace,
						kind: 'script',
						isTemplate: undefined,
						refreshCount: 0
					},
					invalidateMs: 1000 * 60
				}
			: {}
	)
</script>

<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, untrack } from 'svelte'
	import { FlowService, ScriptService } from '$lib/gen'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { Skeleton } from '$lib/components/common'
	import { createCache, emptyString } from '$lib/utils'
	import { Code2 } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { get } from 'svelte/store'
	import { userStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'

	type Item = {
		path: string
		summary?: string
		description?: string
		hash?: string
	}

	let items = usePromise(
		async () =>
			await loadItemsCached({ workspace: $workspaceStore!, kind, isTemplate, refreshCount }),
		{ loadInit: false, clearValueOnRefresh: false }
	)

	let filteredItems: (Item & { marked?: string })[] | undefined = $state(undefined)

	interface Props {
		kind?: 'script' | 'trigger' | 'approval' | 'failure' | 'flow' | 'preprocessor'
		isTemplate?: boolean | undefined
		selected?: number | undefined
		displayPath?: boolean
		filteredWithOwner?: (Item & { marked?: string })[] | undefined
		filter?: string
		owners?: string[]
		ownerFilter?:
			| { kind: 'inline' | 'owner' | 'integrations'; name: string | undefined }
			| undefined
		refreshCount?: number
	}

	let {
		kind = 'script',
		isTemplate = undefined,
		selected = undefined,
		displayPath = false,
		filteredWithOwner = $bindable(undefined),
		filter = '',
		owners = $bindable([]),
		ownerFilter = $bindable(undefined),
		refreshCount = 0
	}: Props = $props()

	const dispatch = createEventDispatcher()
	let lockHash = false

	function onKeyDown(e: KeyboardEvent) {
		if (
			selected != undefined &&
			filteredWithOwner &&
			selected >= 0 &&
			selected < filteredWithOwner.length &&
			e.key === 'Enter'
		) {
			e.preventDefault()
			let item = filteredWithOwner[selected]
			if (kind == 'flow') {
				dispatch('pickFlow', { path: item.path })
			} else {
				dispatch('pickScript', { path: item.path, hash: lockHash ? item.hash : undefined, kind })
			}
		}
	}
	$effect(() => {
		refreshCount
		$workspaceStore && kind && untrack(() => items.refresh())
	})
	$effect(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})
	$effect(() => {
		if (filteredItems) {
			owners = Array.from(
				new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
			).sort((a, b) => {
				if (a.startsWith('u/') && !b.startsWith('u/')) return -1
				if (b.startsWith('u/') && !a.startsWith('u/')) return 1

				if (a.startsWith('f/') && !b.startsWith('f/')) return -1
				if (b.startsWith('f/') && !a.startsWith('f/')) return 1

				return a.localeCompare(b)
			})
		}
	})
	$effect(() => {
		filteredWithOwner =
			ownerFilter != undefined
				? filteredItems?.filter((x) => x.path.startsWith(ownerFilter?.name!))
				: filteredItems
	})
</script>

<SearchItems
	{filter}
	items={items.value}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>

<svelte:window onkeydown={onKeyDown} />
{#if filteredItems}
	{#if filteredItems.length == 0}
		<div class="text-2xs text-primary font-light text-center py-2 px-3 items-center">
			{kind == 'flow' ? 'No flows found.' : 'No scripts found.'}
		</div>
	{/if}
	<ul class="gap-1 flex flex-col">
		{#each filteredWithOwner ?? [] as { path, hash, summary, marked }, index}
			<li class="w-full">
				<Popover class="w-full " placement="right" forceOpen={index === selected}>
					{#snippet text()}
						<div class="flex flex-col">
							<div class="text-left text-xs font-normal leading-tight py-0">{summary ?? ''}</div>
							<div class="text-left text-2xs font-normal">
								{path ?? ''}
							</div>
						</div>
					{/snippet}
					<Button
						selected={selected === index}
						variant="subtle"
						unifiedSize="sm"
						btnClasses="justify-start transition-all"
						onClick={() => {
							if (kind == 'flow') {
								dispatch('pickFlow', { path: path })
							} else {
								dispatch('pickScript', { path: path, hash: lockHash ? hash : undefined, kind })
							}
						}}
						startIcon={{
							icon: kind == 'flow' ? BarsStaggered : Code2
						}}
					>
						<div class="flex flex-col grow min-w-0">
							<div class="grow min-w-0 truncate text-left">
								{#if marked}
									{@html marked}
								{:else}
									{!summary || summary.length == 0 ? path : summary}
								{/if}
							</div>
							{#if displayPath && path}
								<div class="grow min-w-0 truncate text-left text-2xs font-thin">
									{path}
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
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [1.5]]} />
	{/each}
{/if}
