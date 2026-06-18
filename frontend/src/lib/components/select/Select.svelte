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
		inputLeadingClasses,
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
		dropdownClass,
		transformInputSelectedText,
		groupBy,
		sortBy,
		onFocus,
		onBlur,
		onClear,
		onCreateItem,
		useContentEditable = false,
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
		dropdownClass?: string
		transformInputSelectedText?: (text: string, value: Value) => string
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onFocus?: () => void
		onBlur?: () => void
		onClear?: () => void
		onCreateItem?: (value: string) => void
		useContentEditable?: boolean
		startSnippet?: Snippet<[{ item: ProcessedItem<Value>; close: () => void }]>
		endSnippet?: Snippet<[{ item: ProcessedItem<Value>; close: () => void }]>
		bottomSnippet?: Snippet<[{ close: () => void }]>
	} = $props()

	let disabled = $derived(_disabled || (loading && !value))
	let iconSize = $derived(ButtonType.UnifiedIconSizes[size])

	let inputEl: HTMLInputElement | HTMLDivElement | undefined = $state()

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

	// contenteditable is owned by the browser while typing, so the text node
	// must be set imperatively — `>{expr}</div>` would race with the browser's
	// own DOM mutations and produce duplicated text.
	$effect(() => {
		const target = open ? filterText : inputText
		if (useContentEditable && inputEl && inputEl.textContent !== target) {
			inputEl.textContent = target
		}
	})
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
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
		<div class="absolute z-10 right-2 h-full flex items-center pointer-events-none">
			<RightIcon size={iconSize} class="text-secondary" />
		</div>
	{/if}

	{#if tooltip}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<Tooltip>{tooltip}</Tooltip>
		</div>
	{/if}

	<!-- svelte-ignore a11y_autofocus -->
	{#if useContentEditable}
		{@const placeholderText =
			loading && !value ? 'Loading...' : value && !showPlaceholderOnOpen ? inputText : placeholder}
		<div
			contenteditable={!disabled}
			role="textbox"
			aria-disabled={disabled}
			tabindex={disabled ? -1 : 0}
			{id}
			style={containerStyle}
			class={twMerge(
				inputBaseClass,
				inputSizeClasses[size],
				ButtonType.UnifiedHeightClasses[size],
				inputBorderClass({ error, forceFocus: open }),
				'w-full whitespace-pre overflow-hidden',
				inputLeadingClasses[size],
				'focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0',
				open ? '' : 'cursor-pointer',
				loading || (clearable && !disabled && value) || RightIcon ? 'pr-7' : '',
				'empty:before:content-[attr(data-placeholder)]',
				!value ? 'empty:before:text-hint' : 'empty:before:text-primary',
				disabled && '!bg-surface-disabled !border-transparent !text-disabled pointer-events-none',
				inputClass ?? ''
			)}
			data-placeholder={placeholderText}
			oninput={(e) => {
				if (!open) open = true
				filterText = e.currentTarget.textContent ?? ''
			}}
			onpointerdown={() => !disabled && (open = true)}
			onkeydown={(e) => {
				if (e.key === 'Enter') {
					e.preventDefault()
				}
			}}
			bind:this={inputEl}
		></div>
	{:else}
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
				loading || (clearable && !disabled && value) || RightIcon ? 'pr-8' : '',
				inputClass ?? ''
			)}
			autocomplete="off"
			oninput={(e) => {
				// Explicitly open dropdown if closed and update filterText
				if (!open) open = true
				filterText = e.currentTarget.value
			}}
			onpointerdown={() => !disabled && (open = true)}
			bind:this={inputEl}
			{id}
		/>
	{/if}
	<SelectDropdown
		class={dropdownClass}
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
