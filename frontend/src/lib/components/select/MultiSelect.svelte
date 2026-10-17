<script lang="ts" generics="Item extends { label?: string; value: any; }">
	import { clickOutside, reorder } from '$lib/utils'
	import { untrack } from 'svelte'
	import { processItems, type ProcessedItem } from './utils.svelte'
	import SelectDropdown from './SelectDropdown.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import DraggableTags from './DraggableTags.svelte'
	import { Search } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import TextInput, {
		inputBaseClass,
		inputBorderClass,
		inputSizeClasses
	} from '../text_input/TextInput.svelte'

	type Value = Item['value']

	let {
		items,
		placeholder = 'Select items',
		searchPlaceholder = 'Search...',
		value = $bindable(),
		class: className = '',
		style,
		listAutoWidth = true,
		disabled = false,
		disablePortal = false,
		createText,
		reorderable = true,
		noItemsMsg,
		selectedUlClass = '',
		placeholderClass = '',
		allowClear = true,
		hideMainClearBtn = false,
		size = 'md',
		id,
		error = false,
		onOpen,
		groupBy,
		sortBy,
		onCreateItem
	}: {
		items?: Item[]
		value: Value[]
		placeholder?: string
		/** Placeholder of the dropdown's search field. Worth setting where the list is there to
		 *  create entries rather than pick from, so the field says what to type. */
		searchPlaceholder?: string
		class?: string
		style?: string
		filterText?: string
		disabled?: boolean
		listAutoWidth?: boolean
		containerStyle?: string
		inputClass?: string
		disablePortal?: boolean
		createText?: string
		reorderable?: boolean
		noItemsMsg?: string
		selectedUlClass?: string
		placeholderClass?: string
		allowClear?: boolean
		hideMainClearBtn?: boolean
		size?: keyof typeof inputSizeClasses
		id?: string
		error?: boolean
		groupBy?: (item: Item) => string
		sortBy?: (a: Item, b: Item) => number
		onOpen?: () => void
		onClear?: () => void
		onCreateItem?: (value: string) => void
	} = $props()

	let filterText = $state<string>('')
	let open = $state<boolean>(false)
	let wrapperEl: HTMLDivElement | undefined = $state()
	const listId = $props.id()
	let searchInputEl: TextInput | undefined = $state()

	let currentValue = $derived(value ?? [])

	// Keyed on `open` too: the closed list stays mounted for its 500ms collapse, so a reopen
	// within it keeps the same input, and an input mounting during it must not take focus.
	$effect(() => {
		if (open) searchInputEl?.focus()
	})

	let processedItems: ProcessedItem<Value>[] = $derived.by(() => {
		let args = { items, createText, filterText, groupBy, onCreateItem, sortBy }
		return untrack(() => processItems(args))
	})

	$effect(() => {
		if (!open) filterText = ''
	})
	$effect(() => {
		open && untrack(() => onOpen?.())
	})

	let showSearch = $derived(processedItems.length - currentValue.length > 0 || !!onCreateItem)

	let valueEntry = $derived(
		currentValue.map(
			(v) => processedItems.find((item) => item.value === v) ?? { value: v, label: v }
		)
	)

	function onAddValue(item: ProcessedItem<Value>) {
		if (item.__is_create && onCreateItem) {
			const before = currentValue.length
			onCreateItem(item.value)
			// Cleared only once the entry exists: it is then in the list, and leaving it in the
			// field would filter the list down to it. A consumer that refused the value (a
			// duplicate, a name it rejects) keeps what was typed, to be corrected.
			if ((value?.length ?? 0) !== before) filterText = ''
		} else {
			value = [...currentValue, item.value]
			// Picking the last option unmounts the search input that held focus.
			if (!showSearch) wrapperEl?.focus()
		}
	}
	function onRemoveValue(item: ProcessedItem<Value>) {
		keepKeyboardFocus()
		value = currentValue.filter((v) => v !== item.value)
	}

	function clearValue() {
		keepKeyboardFocus()
		filterText = ''
		value = []
	}

	// The chip and clear buttons unmount once they have done their job, which would drop
	// keyboard focus on <body>. Only a keyboard-focused button hands it back to the field, so
	// a mouse click leaves nothing focused.
	function keepKeyboardFocus() {
		const active = document.activeElement
		if (
			active &&
			active !== wrapperEl &&
			wrapperEl?.contains(active) &&
			active.matches(':focus-visible')
		)
			wrapperEl.focus()
	}

	function closeAndFocusField() {
		open = false
		wrapperEl?.focus()
	}

	// Escape that closes the list must not also close the drawer or popover around the field.
	// Melt popovers listen on `document`, where Svelte handles delegated events of portaled
	// content too, so only stopping immediate propagation keeps the event from them.
	function consumeEscape(e: KeyboardEvent) {
		e.preventDefault()
		e.stopImmediatePropagation()
	}

	// Keys the field acts on stop here: editors around it bind the same keys on `window` (arrows
	// move the selected step or component). Modified keys are left to them (Cmd+Enter runs).
	function onFieldKeydown(e: KeyboardEvent) {
		if (e.target !== wrapperEl || e.ctrlKey || e.metaKey || e.altKey) return
		if (!open && ['Enter', ' ', 'ArrowDown', 'ArrowUp'].includes(e.key)) {
			e.preventDefault()
			e.stopPropagation()
			open = true
		} else if (open && e.key === 'Escape') {
			consumeEscape(e)
			open = false
		} else if (open && e.key === 'Tab') {
			open = false
		}
	}

	function onSearchKeydown(e: KeyboardEvent) {
		// Picking is done by SelectDropdown's window listener. With `disablePortal` the input sits
		// in the caller's <form>, where Enter would otherwise click its first submit button.
		if (e.key === 'Enter') e.preventDefault()
		if (e.key === 'Escape') {
			consumeEscape(e)
			closeAndFocusField()
		} else if (e.key === 'Tab') {
			// The list may be portaled to the end of <body>: moving focus back to the field first
			// lets Tab continue from the field rather than off the end of the page.
			closeAndFocusField()
		}
	}

	export function getFilteredInputText() {
		return filterText
	}
</script>

<div
	bind:this={wrapperEl}
	role="combobox"
	aria-controls={listId}
	aria-expanded={open && !disabled}
	aria-haspopup="listbox"
	aria-disabled={disabled}
	tabindex={disabled ? undefined : 0}
	class={twMerge(
		'flex items-center flex-wrap relative',
		inputBaseClass,
		inputSizeClasses[size],
		inputBorderClass({ forceFocus: open && !disabled, error }),
		disabled ? 'pointer-events-none' : '',
		open && !disabled ? 'open' : '',
		disabled ? 'disabled' : '',
		className
	)}
	{style}
	onpointerup={() => (open = true)}
	onkeydown={onFieldKeydown}
	use:clickOutside={{ onClickOutside: () => (open = false) }}
	{id}
>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->

	{#if currentValue.length === 0}
		<span class={twMerge('text-xs h-full flex items-center flex-1 text-hint', placeholderClass)}>
			{placeholder}
		</span>
	{:else}
		<ul
			class={twMerge(
				'overflow-clip overflow-x-hidden h-full cursor-pointer items-center flex flex-wrap gap-1 py-0.5 flex-1 text-primary',
				selectedUlClass
			)}
			role="list"
		>
			<DraggableTags
				items={valueEntry}
				allowClear={allowClear && !disabled}
				onRemove={onRemoveValue}
				onReorder={reorderable
					? (oldIdx, newIdx) => (value = reorder(currentValue, oldIdx, newIdx))
					: undefined}
			/>
		</ul>
	{/if}
	{#if allowClear && !disabled && !hideMainClearBtn && !!value?.length}
		<CloseButton
			noBg
			class="ml-2 remove-all bg-transparent text-hint"
			small
			on:close={(e) => (clearValue(), e.stopPropagation())}
		/>
	{/if}
	<SelectDropdown
		{disablePortal}
		onSelectValue={onAddValue}
		{open}
		processedItems={processedItems.filter((item) => !currentValue.includes(item.value))}
		value={undefined}
		{disabled}
		{filterText}
		getInputRect={wrapperEl && (() => wrapperEl!.getBoundingClientRect())}
		{listAutoWidth}
		{noItemsMsg}
		class={twMerge(
			'multiselect dropdown',
			open && !disabled ? 'open' : '',
			disabled ? 'disabled' : ''
		)}
		ulClass="options"
		{listId}
	>
		{#snippet header()}
			{#if showSearch}
				<div class="mx-2 mb-1 mt-2 flex items-center relative">
					<TextInput
						bind:this={searchInputEl}
						bind:value={filterText}
						inputProps={{
							// Pressing the field or a chip focuses the field; typing must still filter.
							// Not once the last option is picked, as the input is then unmounting, nor
							// when focus goes to a control outside the field and its list.
							onblur: (e: FocusEvent) => {
								const to = e.relatedTarget as Node | null
								if (
									to &&
									!wrapperEl?.contains(to) &&
									!document.getElementById(listId)?.contains(to)
								)
									return
								if (open && showSearch) searchInputEl?.focus()
							},
							onkeydown: onSearchKeydown,
							placeholder: searchPlaceholder
						}}
						class="!pr-7 !bg-surface"
					/>
					<Search size={16} class="absolute right-2 text-primary" />
				</div>
			{/if}
		{/snippet}
	</SelectDropdown>
</div>
