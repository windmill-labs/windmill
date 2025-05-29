<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from './common/CloseButton.svelte'
	import ConditionalPortal from './common/drawer/ConditionalPortal.svelte'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Please select',
		value = $bindable(),
		filterText: _filterTextBind = $bindable(undefined),
		class: className = '',
		clearable = false,
		listAutoWidth = true,
		disabled = false,
		containerStyle = '',
		inputClass = '',
		disablePortal = false,
		groupBy,
		sortBy,
		onFocus,
		onClear
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
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onFocus?: () => void
		onClear?: () => void
	} = $props()

	let filterText = $state<string>('')
	let open = $state<boolean>(false)
	let keyArrowPos = $state<number | undefined>()
	let inputEl: HTMLInputElement | undefined = $state()

	$effect(() => {
		if (_filterTextBind !== undefined) filterText = _filterTextBind
	})
	$effect(() => {
		if (_filterTextBind !== undefined) _filterTextBind = filterText
	})

	$effect(() => {
		;[filterText, open, processedItems]
		keyArrowPos = open && filterText ? 0 : undefined
	})

	$effect(() => {
		if (filterText) open = true
	})
	$effect(() => {
		if (!open) filterText = ''
	})

	let processedItems: (Item & { __select_group?: string; label: string })[] = $derived.by(() => {
		let items2 =
			items?.map((item) => ({
				...item,
				label: getLabel(item)
			})) ?? []
		if (filterText) {
			items2 = items2.filter((item) => item.label.toLowerCase().includes(filterText.toLowerCase()))
		}
		if (groupBy) {
			items2 =
				items2?.map((item) => ({
					...item,
					__select_group: groupBy(item)
				})) ?? []
		}
		if (sortBy) {
			items2 = items2?.sort(sortBy)
		}
		return items2
	})
	let valueEntry = $derived(value && processedItems?.find((item) => item.value === value))

	function setValue(item: Item) {
		value = item.value
		filterText = ''
		open = false
	}

	function clearValue() {
		filterText = ''
		if (onClear) onClear()
		else value = undefined
	}

	function getLabel(item: Item | undefined): string {
		if (!item) return ''
		if (item.label) return item.label
		if (typeof item.value === 'string') return item.value
		if (typeof item.value == 'number' || typeof item.value == 'boolean')
			return item.value.toString()

		return JSON.stringify(item.value)
	}

	function computeDropdownPos(): { width: number; x: number; y: number } {
		if (!inputEl) return { width: 0, x: 0, y: 0 }
		const r = inputEl?.getBoundingClientRect()
		return { width: r.width, x: r.x, y: r.y + r.height }
	}
	let dropdownPos = $state(computeDropdownPos())
	$effect(() => {
		function updateDropdownPos() {
			dropdownPos = computeDropdownPos()
			if (open) requestAnimationFrame(updateDropdownPos)
		}
		if (open) updateDropdownPos()
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
			setValue(processedItems[keyArrowPos])
		} else {
			keyArrowPos = undefined
			return
		}
		e.preventDefault()
	}}
/>

<div
	class={`relative ${className}`}
	use:clickOutside={{ onClickOutside: () => (open = false) }}
	onpointerdown={() => onFocus?.()}
	onfocus={() => onFocus?.()}
>
	{#if clearable && !disabled && value !== undefined}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<CloseButton noBg small on:close={clearValue} />
		</div>
	{/if}
	<input
		{disabled}
		type="text"
		bind:value={() => filterText, (v) => (filterText = v)}
		placeholder={valueEntry?.label ?? placeholder}
		style={containerStyle}
		class={twMerge(
			'!bg-surface text-ellipsis',
			open ? '' : 'cursor-pointer',
			valueEntry ? '!placeholder-primary' : '',
			clearable && !disabled && value !== undefined ? '!pr-8' : '',
			inputClass ?? ''
		)}
		autocomplete="off"
		onpointerdown={() => (open = true)}
		bind:this={inputEl}
	/>

	<ConditionalPortal condition={!disablePortal}>
		{#if open && !disabled}
			<div
				class="flex flex-col absolute z-[5001] max-h-64 overflow-y-auto bg-surface-secondary text-tertiary text-sm select-none border rounded-lg"
				style="top: {dropdownPos.y}px; left: {dropdownPos.x}px; {listAutoWidth
					? `min-width: ${dropdownPos.width}px;`
					: ''}"
			>
				{#if processedItems?.length === 0}
					<div class="py-8 px-4 text-center text-secondary">No items</div>
				{/if}
				{#each processedItems ?? [] as item, itemIndex}
					{#if (item.__select_group && itemIndex === 0) || processedItems?.[itemIndex - 1]?.__select_group !== item.__select_group}
						<div
							class={twMerge(
								'mx-4 pb-1 mb-2 text-xs font-semibold text-secondary border-b',
								itemIndex === 0 ? 'mt-3' : 'mt-6'
							)}
						>
							{item.__select_group}
						</div>
					{/if}
					<button
						class={twMerge(
							'py-2 px-4 w-full font-normal text-left',
							itemIndex === keyArrowPos ? 'bg-surface-hover' : '',
							item.value === value ? 'bg-surface-selected' : 'hover:bg-surface-hover'
						)}
						onclick={() => setValue(item)}
					>
						{item.label}
					</button>
				{/each}
			</div>
		{/if}
	</ConditionalPortal>
</div>
