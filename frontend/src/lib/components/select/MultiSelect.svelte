<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside, reorder } from '$lib/utils'
	import { untrack } from 'svelte'
	import { processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import DraggableTags from './DraggableTags.svelte'
	import { Search } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Select items',
		value = $bindable(),
		class: className = '',
		style,
		listAutoWidth = true,
		disabled = false,
		disablePortal = false,
		createText,
		reorderable = true,
		noItemsMsg,
		selectedUlClass = '',
		placeholderClass = '',
		allowClear = true,
		onOpen,
		groupBy,
		sortBy,
		onCreateItem
	}: {
		items?: Item[]
		value: Value[]
		placeholder?: string
		class?: string
		style?: string
		filterText?: string
		disabled?: boolean
		listAutoWidth?: boolean
		containerStyle?: string
		inputClass?: string
		disablePortal?: boolean
		createText?: string
		reorderable?: boolean
		noItemsMsg?: string
		selectedUlClass?: string
		placeholderClass?: string
		allowClear?: boolean
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onOpen?: () => void
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
	$effect(() => {
		open && untrack(() => onOpen?.())
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
	class={twMerge(
		'relative min-h-8 flex items-center w-full bg-surface border border-gray-300 rounded-md text-tertiary',
		disabled ? 'pointer-events-none' : '',
		open && !disabled ? 'open' : '',
		disabled ? 'disabled' : '',
		className
	)}
	{style}
	onpointerup={() => (open = true)}
	use:clickOutside={{ onClickOutside: () => (open = false) }}
>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->

	{#if value.length === 0}
		<span class={twMerge('text-sm ml-2 h-full flex items-center flex-1', placeholderClass)}>
			{placeholder}
		</span>
	{:else}
		<ul
			class={twMerge(
				'overflow-clip overflow-x-hidden h-full cursor-pointer items-center flex flex-wrap gap-1 py-0.5 px-0.5 flex-1 text-primary',
				selectedUlClass
			)}
			role="list"
		>
			<DraggableTags
				items={valueEntry}
				{allowClear}
				onRemove={onRemoveValue}
				onReorder={reorderable
					? (oldIdx, newIdx) => (value = reorder(value, oldIdx, newIdx))
					: undefined}
			/>
		</ul>
	{/if}
	{#if allowClear}
		<CloseButton
			noBg
			class="mr-1 remove-all"
			small
			on:close={(e) => (clearValue(), e.stopPropagation())}
		/>
	{/if}
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
		{noItemsMsg}
		class={twMerge(
			'multiselect dropdown',
			open && !disabled ? 'open' : '',
			disabled ? 'disabled' : ''
		)}
		ulClass="options"
	>
		{#snippet header()}
			{#if processedItems.length - value.length > 0 || onCreateItem}
				<div class="mx-2 mb-1 mt-2 flex items-center relative">
					<input
						bind:this={searchInputEl}
						bind:value={filterText}
						onblur={(e) => (e.preventDefault(), searchInputEl?.focus())}
						placeholder="Search"
						class="!pr-7"
					/>
					<Search size={16} class="absolute right-2 text-tertiary" />
				</div>
			{/if}
		{/snippet}
	</SelectDropdown>
</div>
