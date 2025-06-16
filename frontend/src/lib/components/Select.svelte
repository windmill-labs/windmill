<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from './common/CloseButton.svelte'
	import ConditionalPortal from './common/drawer/ConditionalPortal.svelte'
	import { deepEqual } from 'fast-equals'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'

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
	let keyArrowPos = $state<number | undefined>()
	let inputEl: HTMLInputElement | undefined = $state()
	let listEl: HTMLDivElement | undefined = $state()

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

	type ProcessedItem = Item & { __select_group?: string; __is_create?: true; label: string }

	let processedItems: ProcessedItem[] = $derived.by(() => {
		let items2 =
			items?.map((item) => ({
				...item,
				label: getLabel(item)
			})) ?? []
		if (filterText) {
			items2 = items2.filter((item) =>
				item?.label?.toLowerCase().includes(filterText?.toLowerCase())
			)
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
		if (onCreateItem && filterText && !items2.some((item) => item.label === filterText)) {
			items2.push({
				label: createText ?? `Add new: "${filterText}"`,
				value: filterText,
				__is_create: true
			} as any)
		}
		return items2
	})
	let valueEntry = $derived(value && processedItems?.find((item) => item.value === value))

	function setValue(item: ProcessedItem) {
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

	function getLabel(item: Item | undefined): string {
		if (!item) return ''
		if (item.label) return item.label
		if (typeof item.value === 'string') return item.value
		if (typeof item.value == 'number' || typeof item.value == 'boolean')
			return item.value.toString()

		return JSON.stringify(item.value)
	}

	function computeDropdownPos(): { width: number; x: number; y: number } {
		if (!inputEl || !listEl) return { width: 0, x: 0, y: 0 }
		const r = inputEl.getBoundingClientRect()
		const listR = listEl.getBoundingClientRect()
		const openBelow = r.y + r.height + listR.height <= window.innerHeight
		let [x, y] = disablePortal ? [0, 0] : [r.x, r.y]
		if (openBelow) return { width: r.width, x: x, y: y + r.height }
		else {
			return { width: r.width, x: x, y: y - listR.height }
		}
	}
	let dropdownPos = $state(computeDropdownPos())
	$effect(() => {
		function updateDropdownPos() {
			let nPos = computeDropdownPos()
			if (!deepEqual(nPos, dropdownPos)) dropdownPos = nPos
			if (open) requestAnimationFrame(updateDropdownPos)
		}
		if (open) untrack(() => updateDropdownPos())
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

	<ConditionalPortal condition={!disablePortal}>
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
						onclick={() => setValue(item)}
					>
						{item.label}
					</button>
				{/each}
			</div>
		{/if}
	</ConditionalPortal>
</div>
