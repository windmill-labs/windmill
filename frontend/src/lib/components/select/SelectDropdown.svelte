<script lang="ts" generics="T">
	import { untrack, type Snippet } from 'svelte'
	import type { ProcessedItem } from './utils.svelte'
	import { twMerge } from 'tailwind-merge'
	import { PlusIcon } from 'lucide-svelte'
	import GenericDropdown from './GenericDropdown.svelte'
	import { Debounced } from 'runed'

	let {
		processedItems: _processedItems,
		value,
		filterText,
		listAutoWidth = true,
		disabled,
		disablePortal = false,
		open,
		noItemsMsg = 'No items found',
		class: className = '',
		ulClass = '',
		itemLabelWrapperClasses = '',
		itemButtonWrapperClasses = '',
		maxHeight = 256,
		header,
		getInputRect,
		onSelectValue,
		startSnippet,
		endSnippet,
		bottomSnippet,
		highlightFirstOnOpen = false
	}: {
		processedItems?: ProcessedItem<T>[]
		value: T | undefined
		filterText?: string
		listAutoWidth?: boolean
		disabled?: boolean
		disablePortal?: boolean
		open: boolean
		noItemsMsg?: string
		class?: string
		ulClass?: string
		itemLabelWrapperClasses?: string
		itemButtonWrapperClasses?: string
		maxHeight?: number
		header?: Snippet
		getInputRect?: () => DOMRect
		onSelectValue: (item: ProcessedItem<T>) => void
		startSnippet?: Snippet<[{ item: ProcessedItem<T>; close: () => void }]>
		endSnippet?: Snippet<[{ item: ProcessedItem<T>; close: () => void }]>
		bottomSnippet?: Snippet<[{ close: () => void }]>
		/** When true, the first item is highlighted when the dropdown opens (even without filterText) */
		highlightFirstOnOpen?: boolean
	} = $props()

	let processedItems = $derived(
		!filterText
			? _processedItems
			: _processedItems?.filter(
					(item) =>
						item.__is_create || item?.label?.toLowerCase().includes(filterText?.toLowerCase())
				)
	)

	let keyArrowPos = $state<number | undefined>()

	$effect(() => {
		;[open, processedItems]
		untrack(() => (keyArrowPos = open && (filterText || highlightFirstOnOpen) ? 0 : undefined))
	})

	// Expose whether the dropdown is visually open for keyboard nav guard.
	// We mirror the same logic GenericDropdown uses: open && !disabled.
	let isVisible = $derived(open && !disabled)

	// Dirty fix to prevent a rendering bug where the ul is present in the layout but
	// displays as empty. It only happens with overflow-y-auto set.
	let enableOverflowYAuto = new Debounced(() => isVisible, 15)
</script>

<svelte:window
	on:keydown={(e) => {
		if (!isVisible || !processedItems?.length) return
		if (e.key === 'ArrowUp' && keyArrowPos !== undefined && processedItems.length > 0) {
			keyArrowPos = keyArrowPos <= 0 ? undefined : keyArrowPos - 1
		} else if (e.key === 'ArrowDown') {
			if (keyArrowPos === undefined) {
				keyArrowPos = 0
			} else {
				keyArrowPos = Math.min(processedItems.length - 1, keyArrowPos + 1)
			}
		} else if (e.key === 'Enter' && keyArrowPos !== undefined && processedItems?.[keyArrowPos]) {
			onSelectValue(processedItems[keyArrowPos])
		} else {
			keyArrowPos = undefined
			return
		}
		e.preventDefault()
	}}
/>

<GenericDropdown
	{listAutoWidth}
	{disablePortal}
	{open}
	{disabled}
	{maxHeight}
	{getInputRect}
	class={className}
>
	{@render header?.()}
	{#if processedItems?.length === 0}
		<div class="py-8 px-4 text-center text-primary text-xs">{noItemsMsg}</div>
	{/if}
	<ul
		class={twMerge(
			'flex-1 flex flex-col',
			enableOverflowYAuto.current ? 'overflow-y-auto' : '',
			ulClass
		)}
	>
		{#each processedItems ?? [] as item, itemIndex (item.value)}
			{#if (item.__select_group && itemIndex === 0) || processedItems?.[itemIndex - 1]?.__select_group !== item.__select_group}
				<li
					class={twMerge(
						'mx-4 pb-1 mb-2 text-xs font-semibold text-primary border-b border-border-light',
						itemIndex === 0 ? 'mt-3' : 'mt-6'
					)}
				>
					{item.__select_group}
				</li>
			{/if}
			<li>
				<button
					class={twMerge(
						'py-2 px-4 w-full font-normal text-left text-primary text-xs',
						itemIndex === keyArrowPos || item.value === value
							? 'bg-surface-secondary dark:bg-surface-tertiary'
							: 'hover:bg-surface-hover',
						endSnippet || item.__is_create ? 'flex items-center justify-between gap-2' : '',
						itemButtonWrapperClasses,
						item.disabled ? 'cursor-not-allowed text-disabled' : ''
					)}
					onclick={(e) => {
						e.stopImmediatePropagation()
						if (!item.disabled) onSelectValue(item)
					}}
				>
					{@render startSnippet?.({ item, close: () => (open = false) })}
					<span class={itemLabelWrapperClasses}>
						{item.label || '\xa0'}
					</span>
					{#if item.__is_create}
						<PlusIcon class="inline ml-auto" size={16} />
					{:else}
						{@render endSnippet?.({ item, close: () => (open = false) })}
					{/if}
					{#if item.subtitle}
						<div class="text-2xs text-secondary">{item.subtitle}</div>
					{/if}
				</button>
			</li>
		{/each}
	</ul>
	{@render bottomSnippet?.({ close: () => (open = false) })}
</GenericDropdown>
