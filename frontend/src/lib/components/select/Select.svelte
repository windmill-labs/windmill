<script
	lang="ts"
	generics="Item extends { label?: string; value: any; subtitle?: string; disabled?: boolean }"
>
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../common/CloseButton.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { untrack, type Snippet } from 'svelte'
	import { getLabel, processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import { deepEqual } from 'fast-equals'
	import {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '../text_input/TextInput.svelte'
	import { ButtonType } from '../common/button/model'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Please select',
		value = $bindable(),
		filterText = $bindable(''),
		class: className = '',
		clearable = false,
		listAutoWidth = true,
		disabled: _disabled = false,
		containerStyle = '',
		inputClass = '',
		disablePortal = false,
		loading = false,
		error = false,
		autofocus,
		RightIcon,
		createText,
		noItemsMsg,
		open = $bindable(false),
		id,
		itemLabelWrapperClasses,
		itemButtonWrapperClasses,
		size = 'md',
		allowUserInput = false,
		transformInputSelectedText,
		groupBy,
		sortBy,
		onFocus,
		onBlur,
		onClear,
		onCreateItem,
		startSnippet,
		endSnippet,
		bottomSnippet
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
		error?: boolean
		autofocus?: boolean
		RightIcon?: any
		createText?: string
		noItemsMsg?: string
		open?: boolean
		id?: string
		itemLabelWrapperClasses?: string
		itemButtonWrapperClasses?: string
		size?: 'sm' | 'md' | 'lg'
		/** When true, any text typed by the user becomes the value when the dropdown closes */
		allowUserInput?: boolean
		transformInputSelectedText?: (text: string) => string
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onFocus?: () => void
		onBlur?: () => void
		onClear?: () => void
		onCreateItem?: (value: string) => void
		startSnippet?: Snippet<[{ item: ProcessedItem<Value>; close: () => void }]>
		endSnippet?: Snippet<[{ item: ProcessedItem<Value>; close: () => void }]>
		bottomSnippet?: Snippet<[{ close: () => void }]>
	} = $props()

	let disabled = $derived(_disabled || (loading && !value))
	let iconSize = $derived(ButtonType.UnifiedIconSizes[size])

	let inputEl: HTMLInputElement | undefined = $state()

	let processedItems: ProcessedItem<Value>[] = $derived.by(() => {
		let args = { items, createText, filterText, groupBy, onCreateItem, sortBy, currentValue: value }
		return untrack(() => processItems(args))
	})

	let valueEntry = $derived(value && processedItems?.find((item) => deepEqual(item.value, value)))

	let rawLabel = $derived(valueEntry?.label ?? getLabel({ value }) ?? '')

	let displayText = $derived(transformInputSelectedText?.(rawLabel) ?? rawLabel)

	let inputValue = $derived(open ? filterText : displayText)

	let hasFilteredItems = $derived(
		!filterText ||
			processedItems?.some(
				(item) => !item.__is_create && item.label?.toLowerCase().includes(filterText.toLowerCase())
			)
	)

	let dropdownVisible = $derived(open && (!allowUserInput || hasFilteredItems))

	$effect(() => {
		if (filterText) open = true
	})
	$effect(() => {
		if (!open) {
			filterText = ''
		} else {
			untrack(() => {
				if (rawLabel) {
					filterText = rawLabel
				}
			})
		}
	})

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
			<Loader2 size={iconSize} class="animate-spin" />
		</div>
	{:else if clearable && !disabled && value}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<CloseButton
				class="bg-transparent text-secondary hover:text-primary"
				noBg
				small
				onClick={clearValue}
			/>
		</div>
	{:else if RightIcon}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<RightIcon size={iconSize} class="text-secondary" />
		</div>
	{/if}
	<!-- svelte-ignore a11y_autofocus -->
	<input
		{autofocus}
		{disabled}
		type="text"
		value={inputValue}
		placeholder={loading && !value ? 'Loading...' : placeholder}
		style={containerStyle}
		class={twMerge(
			inputBaseClass,
			inputSizeClasses[size],
			ButtonType.UnifiedHeightClasses[size],
			inputBorderClass({ error, forceFocus: open }),
			'w-full',
			open ? '' : 'cursor-pointer',
			'placeholder-hint',
			(clearable || RightIcon) && !disabled && value ? 'pr-8' : '',
			inputClass ?? ''
		)}
		autocomplete="off"
		oninput={(e) => {
			if (!open) open = true
			filterText = e.currentTarget.value
			if (allowUserInput) {
				value = filterText || undefined
			}
		}}
		onpointerdown={() => (open = true)}
		bind:this={inputEl}
		{id}
	/>
	<SelectDropdown
		{disablePortal}
		onSelectValue={setValue}
		open={dropdownVisible}
		{processedItems}
		{value}
		{disabled}
		{filterText}
		getInputRect={inputEl && (() => inputEl!.getBoundingClientRect())}
		{listAutoWidth}
		{noItemsMsg}
		{itemLabelWrapperClasses}
		{itemButtonWrapperClasses}
		{startSnippet}
		{endSnippet}
		{bottomSnippet}
	/>
</div>
