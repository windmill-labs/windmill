<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside } from '$lib/utils'
	import { untrack } from 'svelte'
	import { processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import CloseButton from '../common/CloseButton.svelte'

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
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onFocus?: () => void
		onBlur?: () => void
		onClear?: () => void
		onCreateItem?: (value: string) => void
	} = $props()

	let filterText = $state<string>('')
	let open = $state<boolean>(false)
	let triggerEl: HTMLDivElement | undefined = $state()

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
	class={`relative flex items-center w-full  ${className}`}
	use:clickOutside={{ onClickOutside: () => (open = false) }}
	onpointerdown={() => onFocus?.()}
	onfocus={() => onFocus?.()}
	onblur={() => onBlur?.()}
>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		bind:this={triggerEl}
		class="bg-surface w-full min-h-8 rounded-md cursor-pointer items-center flex gap-1 py-0.5 px-0.5"
		onclick={() => (open = true)}
		role="list"
	>
		{#if value.length === 0}
			<span class="text-sm ml-2 text-tertiary">{placeholder}</span>
		{/if}
		{#each valueEntry ?? [] as item}
			<div class="pl-3 pr-1 bg-surface-secondary rounded-full flex items-center gap-0.5">
				<span class="text-sm">{item.label || item.value}</span>
				<CloseButton small on:close={(e) => (onRemoveValue(item), e.stopPropagation())} />
			</div>
		{/each}
	</div>
	<CloseButton class="mr-1" small on:close={(e) => (clearValue(), e.stopPropagation())} />
	<SelectDropdown
		{disablePortal}
		onSelectValue={onAddValue}
		{open}
		processedItems={processedItems.filter((item) => !value.includes(item.value))}
		value={undefined}
		{disabled}
		{filterText}
		getInputRect={triggerEl && (() => triggerEl!.getBoundingClientRect())}
		{listAutoWidth}
	/>
</div>
