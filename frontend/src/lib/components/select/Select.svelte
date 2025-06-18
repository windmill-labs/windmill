<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../common/CloseButton.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import { deepEqual } from 'fast-equals'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Please select',
		value = $bindable(),
		filterText: _filterTextBind = $bindable(undefined),
		class: className = '',
		clearable = false,
		listAutoWidth = true,
		disabled: _disabled = false,
		containerStyle = '',
		inputClass = '',
		disablePortal = false,
		loading = false,
		autofocus,
		RightIcon,
		createText,
		noItemsMsg,
		groupBy,
		sortBy,
		onFocus,
		onBlur,
		onClear,
		onCreateItem
	}: {
		items?: Item[]
		value: Value | undefined
		placeholder?: string
		class?: string
		clearable?: boolean
		filterText?: string
		disabled?: boolean
		listAutoWidth?: boolean
		containerStyle?: string
		inputClass?: string
		disablePortal?: boolean
		loading?: boolean
		autofocus?: boolean
		RightIcon?: any
		createText?: string
		noItemsMsg?: string
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onFocus?: () => void
		onBlur?: () => void
		onClear?: () => void
		onCreateItem?: (value: string) => void
	} = $props()

	let disabled = $derived(_disabled || loading)

	let filterText = $state<string>('')
	let open = $state<boolean>(false)
	let inputEl: HTMLInputElement | undefined = $state()

	let processedItems: ProcessedItem<Value>[] = $derived.by(() => {
		let args = { items, createText, filterText, groupBy, onCreateItem, sortBy }
		return untrack(() => processItems(args))
	})

	$effect(() => {
		if (_filterTextBind !== undefined) filterText = _filterTextBind
	})
	$effect(() => {
		if (_filterTextBind !== undefined) _filterTextBind = filterText
	})

	$effect(() => {
		if (filterText) open = true
	})
	$effect(() => {
		if (!open) filterText = ''
	})

	let valueEntry = $derived(value && processedItems?.find((item) => deepEqual(item.value, value)))

	function setValue(item: ProcessedItem<Value>) {
		if (item.__is_create && onCreateItem) {
			onCreateItem(item.value)
		} else {
			value = item.value
		}
		filterText = ''
		open = false
	}

	function clearValue() {
		filterText = ''
		if (onClear) onClear()
		else value = undefined
	}
</script>

<div
	class={`relative ${className}`}
	use:clickOutside={{ onClickOutside: () => (open = false) }}
	onpointerdown={() => onFocus?.()}
	onfocus={() => onFocus?.()}
	onblur={() => onBlur?.()}
>
	{#if loading}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<Loader2 size={18} class="animate-spin" />
		</div>
	{:else if clearable && !disabled && value}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<CloseButton noBg small on:close={clearValue} />
		</div>
	{:else if RightIcon}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<RightIcon size={18} class="text-tertiary/35" />
		</div>
	{/if}
	<!-- svelte-ignore a11y_autofocus -->
	<input
		{autofocus}
		{disabled}
		type="text"
		bind:value={() => filterText, (v) => (filterText = v)}
		placeholder={loading ? 'Loading...' : (valueEntry?.label ?? placeholder)}
		style={containerStyle}
		class={twMerge(
			'!bg-surface text-ellipsis',
			open ? '' : 'cursor-pointer',
			valueEntry && !loading ? '!placeholder-primary' : '',
			(clearable || RightIcon) && !disabled && value ? '!pr-8' : '',
			inputClass ?? ''
		)}
		autocomplete="off"
		onpointerdown={() => (open = true)}
		bind:this={inputEl}
	/>
	<SelectDropdown
		{disablePortal}
		onSelectValue={setValue}
		{open}
		{processedItems}
		{value}
		{disabled}
		{filterText}
		getInputRect={inputEl && (() => inputEl!.getBoundingClientRect())}
		{listAutoWidth}
		{noItemsMsg}
	/>
</div>
