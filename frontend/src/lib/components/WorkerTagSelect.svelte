<script lang="ts">
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	import { createEventDispatcher } from 'svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import { Button } from './common'
	import { RotateCw } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

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

	let loading = $state(false)
	async function loadWorkerGroups() {
		loading = true
		try {
			if (!$workerTags) {
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

	let open = $state(false)
</script>

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
					loadWorkerGroups()
					open = true
				}}
			></Button>
		</div>
	{/if}
</div>
