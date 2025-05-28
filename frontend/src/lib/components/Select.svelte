<script lang="ts" generics="Item extends { label: string; value: any; }">
	import { clickOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from './common/CloseButton.svelte'

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

	let processedItems: (Item & { __select_group?: string; label: string })[] = $derived.by(() => {
		let items2 =
			items?.map((item) => ({
				...item,
				label: getLabel(item)
			})) ?? []
		if (search) {
			items2 = items2.filter((item) => item.label.toLowerCase().includes(search.toLowerCase()))
			console.log('search', items2)
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

<div class={`relative ${className}`} use:clickOutside={{ onClickOutside: () => (open = false) }}>
	{#if clearable && value !== undefined}
		<div class="absolute z-10 right-2 h-full flex items-center">
			<CloseButton noBg small on:close={clearValue} />
		</div>
	{/if}
	<input
		type="text"
		bind:value={() => search, (v) => (search = v)}
		placeholder={valueEntry?.label ?? placeholder}
		class={twMerge(open ? '' : 'cursor-pointer', valueEntry ? 'placeholder-primary' : '')}
		autocomplete="off"
		onfocus={() => (open = true)}
	/>
	<div class="relative w-full">
		{#if open}
			<div
				class="absolute z-10 max-h-64 overflow-y-scroll w-full bg-surface text-tertiary text-sm select-none border rounded-lg"
			>
				{#each processedItems ?? [] as item, itemIndex}
					{#if (item.__select_group && itemIndex === 0) || processedItems?.[itemIndex - 1].__select_group !== item.__select_group}
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
		{/if}
	</div>
</div>
