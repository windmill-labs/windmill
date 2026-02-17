<script lang="ts">
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../common/CloseButton.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { getLabel, processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '../text_input/TextInput.svelte'
	import { ButtonType } from '../common/button/model'

	type Item = { label?: string; value: string; subtitle?: string; disabled?: boolean }

	let {
		items,
		placeholder = 'Please select',
		value = $bindable(),
		class: className = '',
		clearable = false,
		disabled: _disabled = false,
		containerStyle = '',
		inputClass = '',
		disablePortal = false,
		loading = false,
		error = false,
		autofocus,
		noItemsMsg,
		id,
		size = 'md',
		transformInputSelectedText,
		onFocus,
		onBlur,
		onClose,
		onClear
	}: {
		items?: Item[]
		value: string | undefined
		placeholder?: string
		class?: string
		clearable?: boolean
		disabled?: boolean
		containerStyle?: string
		inputClass?: string
		disablePortal?: boolean
		loading?: boolean
		error?: boolean
		autofocus?: boolean
		noItemsMsg?: string
		id?: string
		size?: 'sm' | 'md' | 'lg'
		transformInputSelectedText?: (text: string) => string
		onFocus?: () => void
		onBlur?: () => void
		onClose?: () => void
		onClear?: () => void
	} = $props()

	let disabled = $derived(_disabled || (loading && !value))
	let iconSize = $derived(ButtonType.UnifiedIconSizes[size])

	let inputEl: HTMLInputElement | undefined = $state()
	let open = $state(false)
	let filterText = $state('')

	let processedItems: ProcessedItem<string>[] = $derived.by(() => {
		let args = { items, filterText }
		return untrack(() => processItems(args))
	})

	let rawLabel = $derived.by(() => {
		let entry = value ? processedItems?.find((item) => item.value === value) : undefined
		return entry?.label ?? getLabel({ value }) ?? ''
	})

	let displayText = $derived(transformInputSelectedText?.(rawLabel) ?? rawLabel)

	let inputValue = $derived(open ? filterText : displayText)

	let hasFilteredItems = $derived(
		!filterText ||
			processedItems?.some((item) => item.label?.toLowerCase().includes(filterText.toLowerCase()))
	)

	let dropdownVisible = $derived(open && hasFilteredItems)

	$effect(() => {
		if (filterText) open = true
	})

	$effect(() => {
		if (!open) {
			filterText = ''
			onClose?.()
		} else {
			untrack(() => {
				if (rawLabel) {
					filterText = rawLabel
				}
			})
		}
	})

	function setValue(item: ProcessedItem<string>) {
		value = item.value
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
			clearable && !disabled && value ? 'pr-8' : '',
			inputClass ?? ''
		)}
		autocomplete="off"
		oninput={(e) => {
			if (!open) open = true
			filterText = e.currentTarget.value
			value = filterText || undefined
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
		{noItemsMsg}
	/>
</div>
