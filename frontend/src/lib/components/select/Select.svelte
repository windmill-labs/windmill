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
	import Tooltip from '../Tooltip.svelte'

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
		tooltip,
		open = $bindable(false),
		id,
		itemLabelWrapperClasses,
		itemButtonWrapperClasses,
		size = 'md',
		showPlaceholderOnOpen = false,
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
		tooltip?: string
		id?: string
		itemLabelWrapperClasses?: string
		itemButtonWrapperClasses?: string
		size?: 'sm' | 'md' | 'lg'
		showPlaceholderOnOpen?: boolean
		transformInputSelectedText?: (text: string, value: Value) => string
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
		let args = { items, createText, filterText, groupBy, onCreateItem, sortBy }
		return untrack(() => processItems(args))
	})

	$effect(() => {
		if (filterText) open = true
	})
	$effect(() => {
		if (!open) filterText = ''
	})

	let valueEntry = $derived(
		value != null ? processedItems?.find((item) => deepEqual(item.value, value)) : undefined
	)

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

	let inputText = $derived.by(() => {
		let text = valueEntry?.label ?? getLabel({ value }) ?? ''
		return transformInputSelectedText?.(text, value) ?? text
	})
</script>

<div
	class={`relative h-fit ${className}`}
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
				on:close={clearValue}
			/>
		</div>
	{:else if RightIcon}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<RightIcon size={iconSize} class="text-secondary" />
		</div>
	{/if}

	{#if tooltip}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<Tooltip>{tooltip}</Tooltip>
		</div>
	{/if}

	<!-- svelte-ignore a11y_autofocus -->
	<input
		{autofocus}
		{disabled}
		type="text"
		bind:value={() => (open ? filterText : inputText), (v) => open && (filterText = v)}
		placeholder={loading && !value
			? 'Loading...'
			: value && !showPlaceholderOnOpen
				? inputText
				: placeholder}
		style={containerStyle}
		class={twMerge(
			inputBaseClass,
			inputSizeClasses[size],
			ButtonType.UnifiedHeightClasses[size],
			inputBorderClass({ error, forceFocus: open }),
			'w-full',
			open ? '' : 'cursor-pointer',
			// Show value as placeholder when opening the dropdown and the search is empty
			!value ? 'placeholder-hint' : '!placeholder-primary',
			(clearable || RightIcon) && !disabled && value ? 'pr-8' : '',
			inputClass ?? ''
		)}
		autocomplete="off"
		oninput={(e) => {
			// Explicitly open dropdown if closed and update filterText
			if (!open) open = true
			filterText = e.currentTarget.value
		}}
		onpointerdown={() => (open = true)}
		bind:this={inputEl}
		{id}
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
		{itemLabelWrapperClasses}
		{itemButtonWrapperClasses}
		{startSnippet}
		{endSnippet}
		{bottomSnippet}
	/>
</div>
