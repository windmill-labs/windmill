<script lang="ts">
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'

	let {
		title = '',
		tooltip = undefined,
		items = $bindable([] as string[]),
		placeholder = 'Add filter (e.g. f/**)'
	} = $props()

	let multiSelect = $state<MultiSelect<{ label?: string; value: any }> | undefined>(undefined)
	const searchText = $derived(multiSelect?.getFilteredInputText())

	function addItem(filter: string) {
		const value = filter.trim()
		if (value && !items.includes(value)) items = [...items, value]
	}
</script>

<div class="flex flex-col gap-1">
	<div class="flex items-center gap-2 mb-1">
		<h4 class="font-semibold text-sm">{title}</h4>
		{#if tooltip}
			{@render tooltip?.()}
		{/if}
	</div>

	<MultiSelect
		bind:this={multiSelect}
		items={safeSelectItems(items)}
		bind:value={items}
		{placeholder}
		searchPlaceholder={placeholder}
		noItemsMsg="Type a path to add it as a filter"
		onCreateItem={addItem}
		createText={searchText ? `Add filter: ${searchText}` : 'Add filter'}
	/>
</div>
