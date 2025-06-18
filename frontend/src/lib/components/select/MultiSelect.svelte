<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside, reorder } from '$lib/utils'
	import { untrack } from 'svelte'
	import { processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import DraggableTags from './DraggableTags.svelte'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Select items',
		value = $bindable(),
		class: className = '',
		listAutoWidth = true,
		disabled = false,
		disablePortal = false,
		createText,
		reorderable = true,
		groupBy,
		sortBy,
		onFocus,
		onBlur,
		onCreateItem
	}: {
		items?: Item[]
		value: Value[]
		placeholder?: string
		class?: string
		filterText?: string
		disabled?: boolean
		listAutoWidth?: boolean
		containerStyle?: string
		inputClass?: string
		disablePortal?: boolean
		createText?: string
		reorderable?: boolean
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onFocus?: () => void
		onBlur?: () => void
		onClear?: () => void
		onCreateItem?: (value: string) => void
	} = $props()

	let filterText = $state<string>('')
	let open = $state<boolean>(false)
	let wrapperEl: HTMLDivElement | undefined = $state()
	let searchInputEl: HTMLInputElement | undefined = $state()

	$effect(() => searchInputEl?.focus())

	let processedItems: ProcessedItem<Value>[] = $derived.by(() => {
		let args = { items, createText, filterText, groupBy, onCreateItem, sortBy }
		return untrack(() => processItems(args))
	})

	$effect(() => {
		if (!open) filterText = ''
	})

	let valueEntry = $derived(
		value.map((v) => processedItems.find((item) => item.value === v)!).filter(Boolean)
	)

	function onAddValue(item: ProcessedItem<Value>) {
		if (item.__is_create && onCreateItem) {
			onCreateItem(item.value)
		} else {
			value = [...value, item.value]
		}
	}
	function onRemoveValue(item: ProcessedItem<Value>) {
		value = value.filter((v) => v !== item.value)
	}

	function clearValue() {
		filterText = ''
		value = []
	}
</script>

<div
	bind:this={wrapperEl}
	class={`relative flex items-center w-full  ${className}`}
	use:clickOutside={{ onClickOutside: () => (open = false) }}
	onpointerdown={() => onFocus?.()}
	onfocus={() => onFocus?.()}
	onblur={() => onBlur?.()}
>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="bg-surface overflow-clip w-full min-h-8 rounded-md cursor-pointer items-center flex flex-wrap gap-1 py-0.5 px-0.5"
		onclick={() => (open = true)}
		role="list"
	>
		{#if value.length === 0}
			<span class="text-sm ml-2 text-tertiary">{placeholder}</span>
		{:else}
			<DraggableTags
				items={valueEntry}
				onRemove={onRemoveValue}
				onReorder={reorderable
					? (oldIdx, newIdx) => (value = reorder(value, oldIdx, newIdx))
					: undefined}
			/>
		{/if}
	</div>
	<CloseButton noBg class="mr-1" small on:close={(e) => (clearValue(), e.stopPropagation())} />
	<SelectDropdown
		{disablePortal}
		onSelectValue={onAddValue}
		{open}
		processedItems={processedItems.filter((item) => !value.includes(item.value))}
		value={undefined}
		{disabled}
		{filterText}
		getInputRect={wrapperEl && (() => wrapperEl!.getBoundingClientRect())}
		{listAutoWidth}
	>
		{#snippet header()}
			{#if processedItems.length - value.length > 0 || onCreateItem}
				<div class="mx-2 mb-1 mt-2">
					<input
						bind:this={searchInputEl}
						bind:value={filterText}
						onblur={(e) => (e.preventDefault(), searchInputEl?.focus())}
						placeholder="Search"
					/>
				</div>
			{/if}
		{/snippet}
	</SelectDropdown>
</div>
