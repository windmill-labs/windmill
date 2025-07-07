<script lang="ts">
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	import { createEventDispatcher } from 'svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'

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
		<div class="text-tertiary text-2xs">{placeholder ?? 'tag'}</div>
	{/if}
	<Select
		clearable
		class="w-full"
		{inputClass}
		{disabled}
		placeholder={nullTag ? nullTag : (placeholder ?? 'lang default')}
		items={safeSelectItems(items)}
		bind:value={() => tag, (value) => ((tag = value), dispatch('change', value))}
	/>
</div>
