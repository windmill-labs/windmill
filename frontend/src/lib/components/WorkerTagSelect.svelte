<script lang="ts">
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import { Button } from './common'
	import { RotateCw } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import Popover from './Popover.svelte'

	let {
		tag = $bindable(),
		noLabel = false,
		nullTag = undefined,
		disabled = false,
		placeholder,
		inputClass,
		workspaceId = undefined
	}: {
		tag: string | undefined
		noLabel?: boolean
		nullTag?: string | undefined
		disabled?: boolean
		placeholder?: string
		language?: string
		class?: string
		inputClass?: string
		// Workspace to read custom tags and worker availability from. Defaults to
		// $workspaceStore. Session editors act on a workspace that differs from the
		// navigation one, so they pass their effective workspace to keep the tag
		// list and availability dots matching the deploy target.
		workspaceId?: string
	} = $props()

	let loading = $state(false)
	let visible = $state(false)
	let timeout: number | undefined = undefined
	let tagsToWorkerExists = $state<Record<string, boolean> | undefined>(undefined)

	// The shared `workerTags` store caches tags for the navigation workspace. When
	// this select targets a different workspace, read/write a local list instead so
	// it neither shows the navigation workspace's tags nor clobbers the shared cache.
	let effectiveWorkspace = $derived(workspaceId ?? $workspaceStore)
	let usesLocal = $derived(workspaceId != undefined && workspaceId !== $workspaceStore)
	let localWorkerTags = $state<string[] | undefined>(undefined)
	let currentTags = $derived(usesLocal ? localWorkerTags : $workerTags)

	onMount(() => {
		visible = true
	})

	onDestroy(() => {
		visible = false
		if (timeout) {
			clearTimeout(timeout)
		}
	})

	loadWorkerGroups()

	const dispatch = createEventDispatcher()

	async function loadWorkerGroups(force = false) {
		loading = true
		try {
			if (usesLocal) {
				if (!localWorkerTags || force) {
					localWorkerTags = await WorkerService.getCustomTagsForWorkspace({
						workspace: effectiveWorkspace!
					})
				}
			} else if (!$workerTags || force) {
				$workerTags = await WorkerService.getCustomTagsForWorkspace({
					workspace: effectiveWorkspace!
				})
			}
		} catch (e) {
			if (usesLocal) {
				localWorkerTags = []
			} else {
				$workerTags = []
			}
			sendUserToast('Error loading custom tags', true)
		}
		loading = false
	}
	let items = $derived([
		// ...(tag ? ['reset to default'] : [nullTag ? `default: ${nullTag}` : '']),
		...(tag && tag != '' && !(currentTags ?? []).includes(tag) ? [tag] : []),
		...(currentTags ?? [])
	])

	let lastCheck: number | undefined = undefined
	async function loadTagsToWorkerExists(tags: string[]) {
		if (lastCheck && Date.now() - lastCheck < 5000) {
			return
		}
		if (timeout) {
			clearTimeout(timeout)
		}
		if (open) {
			tagsToWorkerExists = await WorkerService.existsWorkersWithTags({
				tags: tags.join(','),
				workspace: effectiveWorkspace
			})
			lastCheck = Date.now()
			if (visible) {
				timeout = setTimeout(() => {
					loadTagsToWorkerExists(tags)
				}, 5000)
			}
		}
	}

	// let finalItems = $derived(
	// 	items.map((item) => {
	// 		if (tagsToWorkerExists) {
	// 			return {
	// 				value: item,
	// 				__select_group: tagsToWorkerExists[item]
	// 					? `${placeholder ?? 'Worker'}s available`
	// 					: `No ${placeholder ?? 'Worker'}s`
	// 			}
	// 		}
	// 		return item
	// 	})
	// )

	$effect(() => {
		if (currentTags && open) {
			loadTagsToWorkerExists(currentTags)
		}
	})

	let open = $state(false)
</script>

{#snippet startSnippet({ item })}
	{#if tagsToWorkerExists}
		{#if tagsToWorkerExists[item.value]}
			<Popover>
				{#snippet text()}
					At least one worker with this tag exists and is running.
				{/snippet}
				<div class="rounded-full inline-block bg-green-500 text-white text-xs w-2 h-2 mr-1"></div>
			</Popover>
		{:else}
			<Popover>
				{#snippet text()}
					No workers with this tag exist or is running.
				{/snippet}
				<div class="rounded-full inline-block bg-red-500 text-white text-xs w-2 h-2 mr-1"></div>
			</Popover>
		{/if}
	{/if}
{/snippet}
<div class="flex gap-1 items-center relative">
	{#if !noLabel}
		<div class="text-primary text-xs">{placeholder ?? 'tag'}</div>
	{/if}
	<Select
		clearable
		class="w-full"
		bind:open
		{inputClass}
		{disabled}
		placeholder={nullTag ? nullTag : (placeholder ?? 'lang default')}
		items={safeSelectItems(items)}
		bind:value={() => tag, (value) => ((tag = value), dispatch('change', value))}
		{startSnippet}
		bottomSnippet={refreshAll}
	/>
</div>

{#snippet refreshAll()}
	<Button
		iconOnly
		variant="subtle"
		unifiedSize="sm"
		startIcon={{ icon: RotateCw, classes: loading ? 'animate-spin' : '' }}
		on:click={async () => {
			loadWorkerGroups(true)
			open = true
		}}
		btnClasses="rounded-none"
		title="Refresh worker groups"
	></Button>
{/snippet}
