<script lang="ts">
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	import { createEventDispatcher } from 'svelte'
	import Select from './select/Select.svelte'

	let {
		tag = $bindable(),
		noLabel = false,
		nullTag = undefined,
		disabled = false
	} = $props<{
		tag: string | undefined
		noLabel?: boolean
		nullTag?: string | undefined
		disabled?: boolean
	}>()

	loadWorkerGroups()

	const dispatch = createEventDispatcher()
	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags({ workspace: $workspaceStore })
		}
	}
	let items = $derived([
		// ...(tag ? ['reset to default'] : [nullTag ? `default: ${nullTag}` : '']),
		...(tag && tag != '' && !($workerTags ?? []).includes(tag) ? [tag] : []),
		...($workerTags ?? [])
	])
</script>

<div class="flex gap-1 items-center">
	{#if !noLabel}
		<div class="text-tertiary text-2xs">tag</div>
	{/if}
	<Select
		clearable
		class="w-full"
		{disabled}
		placeholder={nullTag ? `default: ${nullTag}` : 'lang default'}
		items={items.map((value) => ({ value }))}
		bind:value={() => tag, (value) => ((tag = value), dispatch('change', value))}
	/>
</div>
