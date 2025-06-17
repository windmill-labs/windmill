<script lang="ts" generics="T">
	import { deepEqual } from 'fast-equals'
	import ConditionalPortal from '../common/drawer/ConditionalPortal.svelte'
	import { untrack, type Snippet } from 'svelte'
	import type { ProcessedItem } from './utils.svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		processedItems: _processedItems,
		value,
		filterText,
		listAutoWidth = true,
		disabled,
		disablePortal = false,
		open,
		header,
		getInputRect,
		onSelectValue
	}: {
		processedItems?: ProcessedItem<T>[]
		value: T | undefined
		filterText?: string
		listAutoWidth?: Boolean
		disabled?: boolean
		disablePortal?: boolean
		open: boolean
		header?: Snippet
		getInputRect?: () => DOMRect
		onSelectValue: (item: ProcessedItem<T>) => void
	} = $props()

	let processedItems = $derived(
		!filterText
			? _processedItems
			: _processedItems?.filter((item) =>
					item?.label?.toLowerCase().includes(filterText?.toLowerCase())
				)
	)

	let listEl: HTMLDivElement | undefined = $state()
	let dropdownPos = $state(computeDropdownPos())
	let keyArrowPos = $state<number | undefined>()

	function computeDropdownPos(): { width: number; x: number; y: number } {
		if (!getInputRect || !listEl) return { width: 0, x: 0, y: 0 }
		let inputR = getInputRect()
		const listR = listEl.getBoundingClientRect()
		const openBelow = inputR.y + inputR.height + listR.height <= window.innerHeight
		let [x, y] = disablePortal ? [0, 0] : [inputR.x, inputR.y]
		if (openBelow) return { width: inputR.width, x: x, y: y + inputR.height }
		else {
			return { width: inputR.width, x: x, y: y - listR.height }
		}
	}

	$effect(() => {
		function updateDropdownPos() {
			let nPos = computeDropdownPos()
			if (!deepEqual(nPos, dropdownPos)) dropdownPos = nPos
			if (open) requestAnimationFrame(updateDropdownPos)
		}
		if (open) untrack(() => updateDropdownPos())
	})

	$effect(() => {
		;[open, processedItems]
		untrack(() => (keyArrowPos = open && filterText ? 0 : undefined))
	})
</script>

<svelte:window
	on:keydown={(e) => {
		if (!open || !processedItems?.length) return
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

<ConditionalPortal condition={!disablePortal} name="select-dropdown-portal">
	{#if open && !disabled}
		<div
			class={twMerge(
				disablePortal ? 'absolute' : 'fixed',
				'flex flex-col z-[5001] max-h-64 overflow-y-auto bg-surface-secondary text-tertiary text-sm select-none border rounded-lg'
			)}
			style="{`top: ${dropdownPos.y}px; left: ${dropdownPos.x}px;`} {listAutoWidth
				? `min-width: ${dropdownPos.width}px;`
				: ''}"
			bind:this={listEl}
		>
			{@render header?.()}
			{#if processedItems?.length === 0}
				<div class="py-8 px-4 text-center text-primary">No items</div>
			{/if}
			{#each processedItems ?? [] as item, itemIndex}
				{#if (item.__select_group && itemIndex === 0) || processedItems?.[itemIndex - 1]?.__select_group !== item.__select_group}
					<div
						class={twMerge(
							'mx-4 pb-1 mb-2 text-xs font-semibold text-primary border-b',
							itemIndex === 0 ? 'mt-3' : 'mt-6'
						)}
					>
						{item.__select_group}
					</div>
				{/if}
				<button
					class={twMerge(
						'py-2 px-4 w-full font-normal text-left text-primary',
						itemIndex === keyArrowPos ? 'bg-surface-hover' : '',
						item.value === value ? 'bg-surface-selected' : 'hover:bg-surface-hover'
					)}
					onclick={(e) => {
						e.stopImmediatePropagation()
						onSelectValue(item)
					}}
				>
					{item.label}
				</button>
			{/each}
		</div>
	{/if}
</ConditionalPortal>
