<script lang="ts" generics="Item extends { label: string; value: any; }">
	import { twMerge } from 'tailwind-merge'
	import CloseButton from './common/CloseButton.svelte'
	import Popover from './meltComponents/Popover.svelte'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Please select',
		value = $bindable(),
		class: className = '',
		clearable = false,
		groupBy,
		sortBy
	}: {
		items?: Item[]
		value: Value | undefined
		placeholder?: string
		class?: string
		clearable?: boolean
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
	} = $props()

	let search = $state<string>('')
	let open = $state<boolean>(false)
	let keyArrowPos = $state<number | undefined>()

	$effect(() => {
		;[search, open, processedItems]
		keyArrowPos = undefined
	})

	$effect(() => {
		if (search) open = true
	})

	let processedItems: (Item & { __select_group?: string; label: string })[] = $derived.by(() => {
		let items2 = items?.map((item) => ({ ...item, label: getLabel(item) })) ?? []
		if (search)
			items2 = items2.filter((item) => item.label.toLowerCase().includes(search.toLowerCase()))
		if (groupBy) items2 = items2?.map((item) => ({ ...item, __select_group: groupBy(item) })) ?? []
		if (sortBy) items2 = items2?.sort(sortBy)
		return items2
	})
	let valueEntry = $derived(value && processedItems?.find((item) => item.value === value))

	function setValue(item: Item) {
		value = item.value
		search = ''
		open = false
	}

	function clearValue() {
		value = undefined
		search = ''
	}

	function getLabel(item: Item | undefined): string {
		if (!item) return ''
		if (item.label) return item.label
		if (typeof item.value === 'string') return item.value
		if (typeof item.value == 'number' || typeof item.value == 'boolean')
			return item.value.toString()

		return JSON.stringify(item.value)
	}
</script>

<svelte:window
	on:keydown={(e) => {
		if (!open || !processedItems?.length) return
		if (e.key === 'ArrowUp' && keyArrowPos !== undefined) {
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

<Popover
	placement="bottom-start"
	closeOnOtherPopoverOpen
	bind:isOpen={open}
	forceContentToTriggerWidth
	class={twMerge(className, valueEntry ? '!placeholder-primary' : '')}
	usePointerDownOutside
>
	<svelte:fragment slot="trigger">
		<div class={`relative`}>
			{#if clearable && value !== undefined}
				<div class="absolute z-10 right-2 h-full flex items-center">
					<CloseButton noBg small on:close={clearValue} />
				</div>
			{/if}
			<input
				type="text"
				bind:value={() => search, (v) => (search = v)}
				placeholder={valueEntry?.label ?? placeholder}
				autocomplete="off"
			/>
		</div>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div
			class="flex flex-col max-h-64 overflow-y-scroll w-full bg-surface text-tertiary text-sm select-none rounded-lg"
		>
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
						item.value === value
							? 'bg-surface-selected-inverse text-primary-inverse'
							: 'hover:bg-surface-hover'
					)}
					onclick={() => setValue(item)}
				>
					{item.label}
				</button>
			{/each}
		</div>
	</svelte:fragment>
</Popover>
