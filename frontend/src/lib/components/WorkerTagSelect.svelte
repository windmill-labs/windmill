<script lang="ts">
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	import Select from './apps/svelte-select/lib/index'
	import { createEventDispatcher } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'

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
	let darkMode: boolean = $state(false)
</script>

<DarkModeObserver bind:darkMode />

<div class="flex gap-1 items-center">
	{#if !noLabel}
		<div class="text-tertiary text-2xs">tag</div>
	{/if}
	<Select
		{disabled}
		placeholder={nullTag ? `default: ${nullTag}` : 'lang default'}
		containerStyles={'--height: 30px !important; --input-padding: 0px !important; ' +
			(darkMode
				? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
				: SELECT_INPUT_DEFAULT_STYLE.containerStyles)}
		{items}
		value={tag ? { value: tag, label: tag } : undefined}
		on:clear={() => {
			tag = undefined
			dispatch('change', undefined)
		}}
		on:change={(e) => {
			console.log('change', e.detail)
			dispatch('change', e.detail.value)
		}}
		bind:justValue={tag}
	/>

	<!-- <select
		class="min-w-32"
		placeholder="Tag"
		bind:value={tag}
		onchange={(e) => {
			if (tag == '') {
				tag = undefined
			}
			dispatch('change', tag)
		}}
		{disabled}
	>
		{#if tag}
			<option value="">reset to default</option>
		{:else}
			<option value={undefined} disabled selected>{nullTag ? `default: ${nullTag}` : ''}</option>
		{/if}
		{#if tag && tag != '' && !($workerTags ?? []).includes(tag)}
			<option value={tag} selected>{tag}</option>
		{/if}
		{#each $workerTags ?? [] as tag (tag)}
			<option value={tag}>{tag}</option>
		{/each}
	</select> -->
</div>
