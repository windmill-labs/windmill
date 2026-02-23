<script lang="ts">
	import SearchItems from '../SearchItems.svelte'
	import SelectDropdown from '../select/SelectDropdown.svelte'
	import type { ProcessedItem } from '../select/utils.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { Search } from 'lucide-svelte'
	import {
		extractMarkedLabel,
		type SearchableSettingItem
	} from '../instanceSettings'
	import { twMerge } from 'tailwind-merge'
	import { untrack } from 'svelte'

	let {
		searchableItems,
		onSelect,
		class: className = ''
	}: {
		searchableItems: SearchableSettingItem[]
		onSelect: (item: SearchableSettingItem) => void
		class?: string
	} = $props()

	let settingsSearchFilter = $state('')
	let debouncedSearchFilter = $state('')
	let filteredSearchItems: (SearchableSettingItem & { marked: string })[] = $state([])
	let searchInputEl: HTMLDivElement | undefined = $state()

	// Debounce search to avoid running uFuzzy on every keystroke
	$effect(() => {
		const val = settingsSearchFilter
		const timeout = setTimeout(() => untrack(() => (debouncedSearchFilter = val)), 150)
		return () => clearTimeout(timeout)
	})

	const searchDropdownOpen = $derived(
		settingsSearchFilter.trim().length > 0 && filteredSearchItems.length > 0
	)

	let searchProcessedItems: ProcessedItem<SearchableSettingItem & { marked: string }>[] = $derived(
		filteredSearchItems.map((item) => ({
			label: item.label,
			value: item,
			subtitle: item.category
		}))
	)

	function handleSelect(item: SearchableSettingItem) {
		settingsSearchFilter = ''
		onSelect(item)
	}
</script>

<SearchItems
	filter={debouncedSearchFilter}
	items={searchableItems}
	bind:filteredItems={filteredSearchItems}
	f={(x) => x.label + ' ' + (x.description ?? '') + ' ' + x.category}
/>

<div class={twMerge('relative', className)}>
	<div bind:this={searchInputEl} class="relative w-full">
		<Search class="absolute left-2 top-1/2 -translate-y-1/2 text-tertiary" size={14} />
		<TextInput
			inputProps={{ placeholder: 'Search settings...' }}
			bind:value={settingsSearchFilter}
			class="pl-7 text-xs w-full"
		/>
	</div>
	<SelectDropdown
		processedItems={searchProcessedItems}
		value={undefined}
		open={searchDropdownOpen}
		disablePortal
		class="max-w-full"
		itemLabelWrapperClasses="hidden"
		itemButtonWrapperClasses="overflow-hidden"
		getInputRect={searchInputEl ? () => searchInputEl!.getBoundingClientRect() : undefined}
		onSelectValue={(item) => handleSelect(item.value)}
		highlightFirstOnOpen
		maxHeight={400}
	>
		{#snippet startSnippet({ item })}
			<div class="text-xs truncate w-full min-w-0"
				>{@html extractMarkedLabel(item.value.marked, item.value.label.length)}</div
			>
		{/snippet}
	</SelectDropdown>
</div>
