<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, untrack } from 'svelte'
	import { FlowService, ScriptService } from '$lib/gen'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { Skeleton } from '$lib/components/common'
	import { emptyString } from '$lib/utils'
	import { Code2 } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import Popover from '$lib/components/Popover.svelte'

	type Item = {
		path: string
		summary?: string
		description?: string
		hash?: string
	}

	let items: Item[] | undefined = $state(undefined)

	let filteredItems: (Item & { marked?: string })[] | undefined = $state(undefined)

	async function loadItems(): Promise<void> {
		items =
			kind == 'flow'
				? await FlowService.listFlows({ workspace: $workspaceStore! })
				: await ScriptService.listScripts({
						workspace: $workspaceStore!,
						kinds: kind,
						isTemplate
					})
	}

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
	}

	let {
		kind = 'script',
		isTemplate = undefined,
		selected = undefined,
		displayPath = false,
		filteredWithOwner = $bindable(undefined),
		filter = '',
		owners = $bindable([]),
		ownerFilter = $bindable(undefined)
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
		$workspaceStore && kind && untrack(() => loadItems())
	})
	$effect(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})
	$effect(() => {
		owners = Array.from(
			new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
		).sort((a, b) => {
			if (a.startsWith('u/') && !b.startsWith('u/')) return -1
			if (b.startsWith('u/') && !a.startsWith('u/')) return 1

			if (a.startsWith('f/') && !b.startsWith('f/')) return -1
			if (b.startsWith('f/') && !a.startsWith('f/')) return 1

			return a.localeCompare(b)
		})
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
	{items}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>

<svelte:window onkeydown={onKeyDown} />
{#if filteredItems}
	{#if filteredItems.length == 0}
		<div class="text-2xs text-tertiary font-light text-center py-2 px-3 items-center">
			{kind == 'flow' ? 'No flows found.' : 'No scripts found.'}
		</div>
	{/if}
	<ul>
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
					<button
						class="px-3 py-2 gap-2 flex flex-row w-full hover:bg-surface-hover transition-all items-center rounded-md {index ===
						selected
							? 'bg-surface-hover'
							: ''}"
						onclick={() => {
							if (kind == 'flow') {
								dispatch('pickFlow', { path: path })
							} else {
								dispatch('pickScript', { path: path, hash: lockHash ? hash : undefined, kind })
							}
						}}
					>
						{#if kind == 'flow'}
							<BarsStaggered size={14} class="shrink-0" />
						{:else}
							<Code2 size={14} />
						{/if}
						<div class="flex flex-col grow min-w-0">
							<div class="grow min-w-0 truncate text-left text-2xs text-primary font-normal">
								{#if marked}
									{@html marked}
								{:else}
									{!summary || summary.length == 0 ? path : summary}
								{/if}
							</div>
							{#if displayPath && path}
								<div class="grow min-w-0 truncate text-left text-2xs text-secondary font-[220]">
									{path}
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
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[0.5, [1.5]]} />
	{/each}
{/if}
