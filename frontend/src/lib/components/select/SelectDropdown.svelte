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
		noItemsMsg = 'No items found',
		class: className = '',
		ulClass = '',
		itemLabelWrapperClasses = '',
		itemButtonWrapperClasses = '',
		header,
		getInputRect,
		onSelectValue,
		startSnippet,
		endSnippet,
		bottomSnippet
	}: {
		processedItems?: ProcessedItem<T>[]
		value: T | undefined
		filterText?: string
		listAutoWidth?: Boolean
		disabled?: boolean
		disablePortal?: boolean
		open: boolean
		noItemsMsg?: string
		class?: string
		ulClass?: string
		itemLabelWrapperClasses?: string
		itemButtonWrapperClasses?: string
		header?: Snippet
		getInputRect?: () => DOMRect
		onSelectValue: (item: ProcessedItem<T>) => void
		startSnippet?: Snippet<[{ item: ProcessedItem<T>; close: () => void }]>
		endSnippet?: Snippet<[{ item: ProcessedItem<T>; close: () => void }]>
		bottomSnippet?: Snippet<[{ close: () => void }]>
	} = $props()

	let processedItems = $derived(
		!filterText
			? _processedItems
			: _processedItems?.filter(
					(item) =>
						item.__is_create || item?.label?.toLowerCase().includes(filterText?.toLowerCase())
				)
	)

	let listEl: HTMLDivElement | undefined = $state()
	let dropdownPos = $state(computeDropdownPos())
	let keyArrowPos = $state<number | undefined>()

	function computeDropdownPos(): {
		width: number
		height: number
		x: number
		y: number
		isBelow: boolean
	} {
		if (!getInputRect || !listEl) return { width: 0, height: 0, x: 0, y: 0, isBelow: true }
		let inputR = getInputRect()
		const listR = listEl.getBoundingClientRect()
		const isBelow = inputR.y + inputR.height + listR.height <= window.innerHeight
		let [x, y] = disablePortal ? [0, 0] : [inputR.x, inputR.y]
		if (isBelow)
			return { width: inputR.width, height: listR.height, x: x, y: y + inputR.height, isBelow }
		else {
			return { width: inputR.width, height: listR.height, x: x, y: y - listR.height, isBelow }
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

	// We do not want to render the dropdown when it is closed for performance reasons
	// but we want to keep it in the DOM for a short time to allow for transitions to finish
	//
	// We do not use Svelte transitions because they can not animate in the opposite direction
	// when the dropdown is opens above the input
	// Also CSS transitions are smoother because they do not rely on JS / animation frames
	let uiState = $state({ domExists: open, visible: open, timeout: null as number | null })
	let initial = true
	$effect(() => {
		let isOpen = open && !disabled
		untrack(() => {
			if (initial) {
				initial = false
				return
			}
			if (uiState.timeout) clearTimeout(uiState.timeout)
			uiState = {
				domExists: true,
				visible: !isOpen,
				timeout: setTimeout(() => {
					if (isOpen) {
						uiState.visible = true
						uiState.timeout = null
					} else if (!isOpen) {
						uiState.visible = false
						uiState.timeout = setTimeout(() => {
							uiState.domExists = false
							uiState.timeout = null
						}, 500) // leave time for transition to finish
					}
				}, 0) // We need the height to be 0 then change immediately for the transition to play
			}
		})
	})
</script>

<svelte:window
	on:keydown={(e) => {
		if (!uiState.visible || !processedItems?.length) return
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
	{#if uiState.domExists}
		<div
			class={twMerge(
				disablePortal ? 'absolute' : 'fixed',
				'z-[5002] text-primary text-sm select-none',
				dropdownPos.isBelow ? '' : 'flex flex-col justify-end',
				uiState.visible ? '' : 'pointer-events-none',
				className
			)}
			style="{`top: ${dropdownPos.y}px; left: ${dropdownPos.x}px;`} {listAutoWidth
				? `min-width: ${dropdownPos.width}px; height: ${dropdownPos.height}px;`
				: ''}"
		>
			<div
				class={twMerge(
					'overflow-clip rounded-md drop-shadow-base transition-height',
					dropdownPos.isBelow ? '' : 'flex flex-col justify-end'
				)}
				style="height: {uiState.visible ? dropdownPos.height : 0}px;"
			>
				<div bind:this={listEl} class="flex flex-col max-h-64 rounded-md bg-surface-tertiary">
					{@render header?.()}
					{#if processedItems?.length === 0}
						<div class="py-8 px-4 text-center text-primary text-xs">{noItemsMsg}</div>
					{/if}
					<ul class={twMerge('flex-1 overflow-y-auto flex flex-col', ulClass)}>
						{#each processedItems ?? [] as item, itemIndex}
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
										itemIndex === keyArrowPos
											? 'bg-surface-secondary'
											: item.value === value
												? 'bg-surface-secondary'
												: 'hover:bg-surface-hover',
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
									{@render endSnippet?.({ item, close: () => (open = false) })}
									{#if item.subtitle}
										<div class="text-2xs text-secondary">{item.subtitle}</div>
									{/if}
								</button>
							</li>
						{/each}
					</ul>
					{@render bottomSnippet?.({ close: () => (open = false) })}
				</div>
			</div>
		</div>
	{/if}
</ConditionalPortal>
