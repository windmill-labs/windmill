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
		inputClass
	}: {
		tag: string | undefined
		noLabel?: boolean
		nullTag?: string | undefined
		disabled?: boolean
		placeholder?: string
		language?: string
		class?: string
		inputClass?: string
	} = $props()

	let loading = $state(false)
	let visible = $state(false)
	let timeout: NodeJS.Timeout | undefined = undefined
	let tagsToWorkerExists = $state<Record<string, boolean> | undefined>(undefined)

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
			if (!$workerTags || force) {
				$workerTags = await WorkerService.getCustomTags({ workspace: $workspaceStore })
			}
		} catch (e) {
			$workerTags = []
			sendUserToast('Error loading custom tags', true)
		}
		loading = false
	}
	let items = $derived([
		// ...(tag ? ['reset to default'] : [nullTag ? `default: ${nullTag}` : '']),
		...(tag && tag != '' && !($workerTags ?? []).includes(tag) ? [tag] : []),
		...($workerTags ?? [])
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
			tagsToWorkerExists = await WorkerService.existsWorkersWithTags({ tags: tags.join(',') })
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
		if ($workerTags && open) {
			loadTagsToWorkerExists($workerTags)
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
		<div class="text-tertiary text-2xs">{placeholder ?? 'tag'}</div>
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
	/>
	{#if open}
		<div class="absolute top-0 -right-12">
			<Button
				iconOnly
				variant="border"
				color="dark"
				size="xs"
				startIcon={{ icon: RotateCw, classes: loading ? 'animate-spin' : '' }}
				on:click={async () => {
					loadWorkerGroups(true)
					open = true
				}}
			></Button>
		</div>
	{/if}
</div>
