<script>
	// @ts-nocheck
	import { beforeUpdate, createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { offset, flip, shift } from '@floating-ui/dom'
	import { createFloatingActions } from 'svelte-floating-ui'

	// This component caused trouble with svelte 5 so better be extra safe
	const dispatch = createDispatcherIfMounted(createEventDispatcher())

	import _filter from './filter'
	import _getItems from './get-items'

	import ChevronIcon from './ChevronIcon.svelte'
	import ClearIcon from './ClearIcon.svelte'
	import LoadingIcon from './LoadingIcon.svelte'
	import ConditionalPortal from './ConditionalPortal.svelte'
	import ConditionalPortalGlobal from './ConditionalPortalGlobal.svelte'

	import { extractCustomProperties, truncate } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	export let portal = true

	export let justValue = null // read-only

	export let inAppEditor = false

	let PortalWrapper = inAppEditor ? ConditionalPortal : ConditionalPortalGlobal

	export let filter = _filter
	export let getItems = _getItems

	export let id = null
	export let name = null
	export let container = undefined
	export let input = undefined

	export let disabled = false
	export let focused = false
	export let value = undefined
	export let filterText = ''
	export let placeholder = 'Please select'
	export let items = undefined
	export let label = 'label'
	export let itemFilter = (label, filterText, option) =>
		`${label}`.toLowerCase().includes(filterText.toLowerCase())
	export let groupBy = undefined
	export let groupFilter = (groups) => groups
	export let groupHeaderSelectable = false
	export let itemId = 'value'
	export let loadOptions = undefined
	export let containerStyles = ''
	export let hasError = false
	export let filterSelectedItems = true
	export let required = false
	export let closeListOnChange = true
	export let computeOnClick = false

	export let createGroupHeaderItem = (groupValue, item) => {
		return {
			value: groupValue,
			[label]: groupValue
		}
	}

	export const getFilteredItems = () => {
		return filteredItems
	}

	export let searchable = true
	export let inputStyles = ''
	export let clearable = true
	export let loading = false
	export let listOpen = false

	let timeout
	export let debounce = (fn, wait = 1) => {
		clearTimeout(timeout)
		timeout = setTimeout(fn, wait)
	}

	export let debounceWait = 300
	export let hideEmptyState = false
	export let inputAttributes = {}
	export let listAutoWidth = true
	export let showChevron = false
	export let listOffset = 5
	export let hoverItemIndex = 0
	export let floatingConfig = {}

	export { containerClasses as class }

	let containerClasses = ''
	let activeValue
	let prev_value
	let prev_filterText
	let hasClicked = false

	function setValue() {
		if (typeof value === 'string') {
			let item = (items || []).find((item) => item[itemId] === value)
			value = item || {
				[itemId]: value,
				label: value
			}
		}
	}

	let _inputAttributes
	function assignInputAttributes() {
		_inputAttributes = Object.assign(
			{
				autocapitalize: 'none',
				autocomplete: 'off',
				autocorrect: 'off',
				spellcheck: false,
				tabindex: 0,
				type: 'text',
				'aria-autocomplete': 'list'
			},
			inputAttributes
		)

		if (id) {
			_inputAttributes['id'] = id
		}

		if (!searchable) {
			_inputAttributes['readonly'] = true
		}
	}

	function convertStringItemsToObjects(_items) {
		return _items.map((item, index) => {
			return {
				index,
				value: item,
				label: `${item}`
			}
		})
	}

	function filterGroupedItems(_items) {
		const groupValues = []
		const groups = {}

		_items.forEach((item) => {
			const groupValue = groupBy(item)

			if (!groupValues.includes(groupValue)) {
				groupValues.push(groupValue)
				groups[groupValue] = []

				if (groupValue) {
					groups[groupValue].push(
						Object.assign(createGroupHeaderItem(groupValue, item), {
							id: groupValue,
							groupHeader: true,
							selectable: groupHeaderSelectable
						})
					)
				}
			}

			groups[groupValue].push(Object.assign({ groupItem: !!groupValue }, item))
		})

		const sortedGroupedItems = []

		groupFilter(groupValues).forEach((groupValue) => {
			if (groups[groupValue]) sortedGroupedItems.push(...groups[groupValue])
		})

		return sortedGroupedItems
	}

	function dispatchSelectedItem() {
		if (!prev_value || JSON.stringify(value[itemId]) !== JSON.stringify(prev_value[itemId])) {
			dispatch('input', value)
		}
	}

	$: if ((items, value)) setValue()
	$: if (inputAttributes || !searchable) assignInputAttributes()
	$: if (value) dispatchSelectedItem()
	$: if (!focused && input) closeList()
	$: if (filterText !== prev_filterText) setupFilterText()
	$: if (listOpen && value && filteredItems) setValueIndexAsHoverIndex()
	$: dispatchHover(hoverItemIndex)

	function setValueIndexAsHoverIndex() {
		const valueIndex = filteredItems.findIndex((i) => {
			return i[itemId] === value[itemId]
		})

		checkHoverSelectable(valueIndex, true)
	}

	function dispatchHover(i) {
		dispatch('hoverItem', i)
	}

	function checkHoverSelectable(startingIndex = 0, ignoreGroup) {
		hoverItemIndex = startingIndex < 0 ? 0 : startingIndex
		if (
			!ignoreGroup &&
			groupBy &&
			filteredItems[hoverItemIndex] &&
			!filteredItems[hoverItemIndex].selectable
		) {
			setHoverIndex(1)
		}
	}

	function setupFilterText() {
		if (computeOnClick && !hasClicked) return
		if (!loadOptions && filterText.length === 0) return

		if (loadOptions) {
			debounce(async function () {
				loading = true
				let res = await getItems({
					dispatch,
					loadOptions,
					convertStringItemsToObjects,
					filterText
				})

				if (res) {
					loading = res.loading
					listOpen = listOpen ? res.listOpen : filterText.length > 0 ? true : false
					focused = listOpen && res.focused
					items = groupBy ? filterGroupedItems(res.filteredItems) : res.filteredItems
				} else {
					loading = false
					focused = true
					listOpen = true
				}
			}, debounceWait)
		} else {
			listOpen = true
		}
	}

	$: hasValue = value
	$: hideSelectedItem = hasValue && filterText.length > 0
	$: showClear = hasValue && clearable && !disabled && !loading
	$: placeholderText = value ? '' : placeholder
	$: ariaSelection = value ? handleAriaSelection() : ''
	$: ariaContext = handleAriaContent({ filteredItems, hoverItemIndex, focused, listOpen })
	$: updateValueDisplay(items)
	$: justValue = computeJustValue(value, itemId)
	$: if (prev_value && !value) dispatch('input', value)
	$: filteredItems = filter({
		loadOptions,
		filterText,
		items,
		value,
		itemId,
		groupBy,
		label,
		filterSelectedItems,
		itemFilter,
		convertStringItemsToObjects,
		filterGroupedItems
	})
	$: if (listOpen && filteredItems && !value) checkHoverSelectable()
	$: handleFilterEvent(filteredItems)

	$: if (container && floatingConfig) floatingUpdate(Object.assign(_floatingConfig, floatingConfig))
	$: listDom = !!list
	$: listMounted(list, listOpen)
	$: if (listOpen && container && list) setListWidth()
	$: scrollToHoverItem = hoverItemIndex
	$: if (input && listOpen && !focused) handleFocus()
	$: if (filterText) hoverItemIndex = 0

	function handleFilterEvent(items) {
		if (listOpen) dispatch('filter', items)
	}

	beforeUpdate(async () => {
		prev_value = value
		prev_filterText = filterText
	})

	function computeJustValue() {
		return value ? value[itemId] : value
	}

	function findItem(selection) {
		let matchTo = selection ? selection[itemId] : value[itemId]
		return items.find((item) => item[itemId] === matchTo)
	}

	function updateValueDisplay(items) {
		if (!items || items.length === 0 || items.some((item) => typeof item !== 'object')) return
		if (!value || !value[itemId]) return

		if (Array.isArray(value)) {
			value = value.map((selection) => findItem(selection) || selection)
		} else {
			value = findItem() || value
		}
	}

	function handleKeyDown(e) {
		if (!focused) return
		e.stopPropagation()
		switch (e.key) {
			case 'Escape':
				e.preventDefault()
				closeList()
				break
			case 'Enter':
				e.preventDefault()

				if (listOpen) {
					if (filteredItems.length === 0) break
					const hoverItem = filteredItems[hoverItemIndex]

					if (value && value[itemId] === hoverItem[itemId]) {
						closeList()
						break
					} else {
						handleSelect(filteredItems[hoverItemIndex])
					}
				}

				break
			case 'ArrowDown':
				e.preventDefault()

				if (listOpen) {
					setHoverIndex(1)
				} else {
					listOpen = true
					activeValue = undefined
				}

				break
			case 'ArrowUp':
				e.preventDefault()

				if (listOpen) {
					setHoverIndex(-1)
				} else {
					listOpen = true
					activeValue = undefined
				}

				break
			case 'Tab':
				if (listOpen && focused) {
					if (
						filteredItems.length === 0 ||
						(value && value[itemId] === filteredItems[hoverItemIndex][itemId])
					)
						return closeList()

					e.preventDefault()
					handleSelect(filteredItems[hoverItemIndex])
					closeList()
				}

				break
			case 'Backspace':
				if (filterText.length > 0) return

				break
			case 'ArrowLeft':
				if (!value || filterText.length > 0) return
				if (activeValue === undefined) {
					activeValue = value.length - 1
				} else if (value.length > activeValue && activeValue !== 0) {
					activeValue -= 1
				}
				break
			case 'ArrowRight':
				if (!value || filterText.length > 0 || activeValue === undefined) return
				if (activeValue === value.length - 1) {
					activeValue = undefined
				} else if (activeValue < value.length - 1) {
					activeValue += 1
				}
				break
		}
	}

	function handleFocus(e) {
		if (focused && input === document?.activeElement) return
		if (e) dispatch('focus', e)
		input.focus()
		focused = true
	}

	async function handleBlur(e) {
		if (listOpen || focused) {
			dispatch('blur', e)
			closeList()
			focused = false
			activeValue = undefined
			input.blur()
		}
	}

	function handleClick() {
		if (disabled) return
		if (computeOnClick && !hasClicked) {
			hasClicked = true
			setupFilterText()
		}
		listOpen = !listOpen
	}

	export function handleClear() {
		dispatch('clear', value)
		value = undefined
		closeList()
		handleFocus()
	}

	onMount(() => {
		if (listOpen) focused = true
		if (focused && input) input.focus()
	})

	function itemSelected(selection) {
		if (selection) {
			filterText = ''
			const item = Object.assign({}, selection)

			if (item.groupHeader && !item.selectable) return
			value = value = item

			setTimeout(() => {
				if (closeListOnChange) closeList()
				activeValue = undefined
				dispatch('change', value)
				dispatch('select', selection)
			})
		}
	}

	function closeList() {
		filterText = ''
		listOpen = false
	}

	export let ariaValues = (values) => {
		return `Option ${values}, selected.`
	}

	export let ariaListOpen = (label, count) => {
		return `You are currently focused on option ${label}. There are ${count} results available.`
	}

	export let ariaFocused = () => {
		return `Select is focused, type to refine list, press down to open the menu.`
	}

	function handleAriaSelection() {
		let selected = undefined
		selected = value[label]

		return ariaValues(selected)
	}

	function handleAriaContent() {
		if (!filteredItems || filteredItems.length === 0) return ''
		let _item = filteredItems[hoverItemIndex]
		if (listOpen && _item) {
			let count = filteredItems ? filteredItems.length : 0
			return ariaListOpen(_item[label], count)
		} else {
			return ariaFocused()
		}
	}

	let list = null

	function handleClickOutside(event) {
		if (
			!listOpen &&
			!focused &&
			container &&
			!container.contains(event.target) &&
			!list?.contains(event.target)
		) {
			handleBlur()
		}
	}

	onDestroy(() => {
		list?.remove()
	})

	function handleSelect(item) {
		if (!item || item.selectable === false) return
		itemSelected(item)
	}

	function handleHover(i) {
		hoverItemIndex = i
	}

	function handleItemClick(args) {
		const { item, i } = args
		if (item?.selectable === false) return
		if (value && value[itemId] === item[itemId]) return closeList()
		if (isItemSelectable(item)) {
			hoverItemIndex = i
			handleSelect(item)
		}
	}

	function setHoverIndex(increment) {
		let selectableFilteredItems = filteredItems.filter(
			(item) => !Object.hasOwn(item, 'selectable') || item.selectable === true
		)

		if (selectableFilteredItems.length === 0) {
			return (hoverItemIndex = 0)
		}

		if (increment > 0 && hoverItemIndex === filteredItems.length - 1) {
			hoverItemIndex = 0
		} else if (increment < 0 && hoverItemIndex === 0) {
			hoverItemIndex = filteredItems.length - 1
		} else {
			hoverItemIndex = hoverItemIndex + increment
		}

		const hover = filteredItems[hoverItemIndex]

		if (hover && hover.selectable === false) {
			if (increment === 1 || increment === -1) setHoverIndex(increment)
			return
		}
	}

	function isItemActive(item, value, itemId) {
		return value && value[itemId] === item[itemId]
	}

	function isItemFirst(itemIndex) {
		return itemIndex === 0
	}

	function isItemSelectable(item) {
		return (
			(item.groupHeader && item.selectable) || item.selectable || !item.hasOwnProperty('selectable')
		)
	}

	function setListWidth() {
		const { width } = container.getBoundingClientRect()
		list.style.width =
			listAutoWidth && list.clientWidth && list.clientWidth < width ? width + 'px' : 'auto'
	}

	let _floatingConfig = {
		strategy: 'absolute',
		placement: 'bottom-start',
		middleware: [offset(listOffset), flip(), shift()],
		autoUpdate: false
	}

	const [floatingRef, floatingContent, floatingUpdate] = createFloatingActions(_floatingConfig)

	$: if (container && floatingConfig?.autoUpdate === undefined) {
		_floatingConfig.autoUpdate = true
	}

	let prefloat = true
	function listMounted(list, listOpen) {
		if (!list || !listOpen) return (prefloat = true)
		setTimeout(() => {
			prefloat = false
		}, 0)
	}
</script>

<svelte:window on:click={handleClickOutside} on:keydown={handleKeyDown} />

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={twMerge('svelte-select', containerClasses)}
	class:disabled
	class:focused
	class:list-open={listOpen}
	class:show-chevron={showChevron}
	class:error={hasError}
	style={containerStyles}
	on:pointerup|preventDefault={handleClick}
	on:mousedown|preventDefault
	bind:this={container}
	use:floatingRef
>
	{#if listOpen}
		<PortalWrapper condition={portal}>
			<div
				style={extractCustomProperties(containerStyles)}
				use:floatingContent
				bind:this={list}
				class="svelte-select-list"
				class:prefloat
				on:pointerup|preventDefault|stopPropagation
				on:pointerdown|preventDefault|stopPropagation
			>
				{#if $$slots['list-prepend']}<slot name="list-prepend" />{/if}
				{#if $$slots.list}<slot name="list" {filteredItems} />
				{:else if filteredItems.length > 0}
					{#each filteredItems as item, i}
						<div
							on:mouseover={() => handleHover(i)}
							on:focus={() => handleHover(i)}
							on:click|stopPropagation={() => handleItemClick({ item, i })}
							on:keydown|preventDefault|stopPropagation
							class="list-item"
							tabindex="-1"
						>
							<div
								class="item"
								class:list-group-title={item.groupHeader}
								class:active={isItemActive(item, value, itemId)}
								class:first={isItemFirst(i)}
								class:hover={hoverItemIndex === i}
								class:group-item={item.groupItem}
								class:not-selectable={item?.selectable === false}
							>
								<slot name="item" {item} index={i}>
									{item ? truncate(item?.[label], 80) : ''}
								</slot>
							</div>
						</div>
					{/each}
				{:else if !hideEmptyState}
					<slot name="empty">
						<div class="empty">No options</div>
					</slot>
				{/if}
				{#if $$slots['list-append']}<slot name="list-append" />{/if}
			</div>
		</PortalWrapper>
	{/if}

	<span aria-live="polite" aria-atomic="false" aria-relevant="additions text" class="a11y-text">
		{#if focused}
			<span id="aria-selection">{ariaSelection}</span>
			<span id="aria-context">
				{ariaContext}
			</span>
		{/if}
	</span>

	<div class="prepend">
		<slot name="prepend" />
	</div>

	<div class="value-container">
		{#if hasValue}
			<div class="selected-item" class:hide-selected-item={hideSelectedItem}>
				<slot name="selection" selection={value}>
					{value[label]}
				</slot>
			</div>
		{/if}

		<input
			on:keydown={handleKeyDown}
			on:blur={handleBlur}
			on:focus={handleFocus}
			readOnly={!searchable}
			{..._inputAttributes}
			bind:this={input}
			bind:value={filterText}
			placeholder={placeholderText}
			style={inputStyles}
		/>
	</div>

	<div class="indicators">
		{#if loading}
			<div class="icon loading" aria-hidden="true">
				<slot name="loading-icon">
					<LoadingIcon />
				</slot>
			</div>
		{/if}

		{#if showClear}
			<button type="button" class="icon clear-select" on:click|stopPropagation={handleClear}>
				<slot name="clear-icon">
					<ClearIcon />
				</slot>
			</button>
		{/if}

		{#if showChevron}
			<div class="icon chevron" aria-hidden="true">
				<slot name="chevron-icon" {listOpen}>
					<ChevronIcon />
				</slot>
			</div>
		{/if}
	</div>

	<slot name="input-hidden" {value}>
		<input {name} type="hidden" value={value ? JSON.stringify(value) : null} />
	</slot>

	{#if required && (!value || value.length === 0)}
		<slot name="required" {value}>
			<select class="required" required tabindex="-1" aria-hidden="true"></select>
		</slot>
	{/if}
</div>

<style>
	.svelte-select {
		border: var(--border, 1px solid #d8dbdf);
		border-radius: var(--border-radius, 6px);
		min-height: var(--height, 42px);
		position: relative;
		display: flex;
		align-items: stretch;
		padding: var(--padding, var(--internal-padding));
		background: var(--background, #fff);
		margin: var(--margin, 0);
		width: var(--width, 100%);
		font-size: var(--font-size, 16px);
		max-height: var(--max-height);
	}

	* {
		box-sizing: var(--box-sizing, border-box);
	}

	.svelte-select:hover {
		border: var(--border-hover, 1px solid #b2b8bf);
	}

	.value-container {
		display: flex;
		flex: 1 1 0%;
		flex-wrap: wrap;
		align-items: center;
		gap: 5px 10px;
		padding: var(--value-container-padding, 5px 0);
		position: relative;
		overflow: var(--value-container-overflow, hidden);
		align-self: stretch;
		padding: 0 !important;
	}

	.prepend,
	.indicators {
		display: flex;
		flex-shrink: 0;
		align-items: center;
	}

	.indicators {
		position: var(--indicators-position);
		top: var(--indicators-top);
		right: var(--indicators-right);
		bottom: var(--indicators-bottom);
	}

	input {
		position: absolute;
		cursor: default;
		border: none;
		color: var(--input-color, var(--item-color));
		padding: var(--input-padding, 0) !important;
		letter-spacing: var(--input-letter-spacing, inherit);
		margin: var(--input-margin, 0) !important;
		min-width: 10px;
		top: 0;
		right: 0;
		bottom: 0;
		left: 0;
		background: transparent !important;
		font-size: var(--font-size, 16px);
		border: 0 !important;
	}

	:not(.multi) > .value-container > input {
		width: 100%;
		height: 100%;
	}

	input::placeholder {
		color: var(--placeholder-color, #78848f);
		opacity: var(--placeholder-opacity, 1);
	}

	input:focus {
		outline: none;
	}

	.svelte-select.focused {
		border: var(--border-focused, 1px solid #006fe8);
		border-radius: var(--border-radius-focused, var(--border-radius, 6px));
	}

	.disabled {
		background: var(--disabled-background, #ebedef);
		border-color: var(--disabled-border-color, #ebedef);
		color: var(--disabled-color, #c1c6cc);
	}

	.disabled input::placeholder {
		color: var(--disabled-placeholder-color, #c1c6cc);
		opacity: var(--disabled-placeholder-opacity, 0.8);
	}

	.selected-item {
		position: relative;
		overflow: var(--selected-item-overflow, hidden);
		padding: var(--selected-item-padding, 0 20px 0 0);
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--selected-item-color, inherit);
		font-size: var(--font-size, 16px);
	}

	.multi .selected-item {
		position: absolute;
		line-height: var(--height, 42px);
		height: var(--height, 42px);
	}

	.selected-item:focus {
		outline: none;
	}

	.hide-selected-item {
		opacity: 0;
	}

	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.clear-select {
		all: unset;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--clear-select-width, 40px);
		height: var(--clear-select-height, 100%);
		color: var(--clear-select-color, var(--icons-color));
		margin: var(--clear-select-margin, 0);
		pointer-events: all;
		flex-shrink: 0;
	}

	.clear-select:focus {
		outline: var(--clear-select-focus-outline, 1px solid #006fe8);
	}

	.loading {
		width: var(--loading-width, 40px);
		height: var(--loading-height);
		color: var(--loading-color, var(--icons-color));
		margin: var(--loading--margin, 0);
		flex-shrink: 0;
	}

	.chevron {
		width: var(--chevron-width, 40px);
		height: var(--chevron-height, 40px);
		background: var(--chevron-background, transparent);
		pointer-events: var(--chevron-pointer-events, none);
		color: var(--chevron-color, var(--icons-color));
		border: var(--chevron-border, 0 0 0 1px solid #d8dbdf);
		flex-shrink: 0;
	}

	.multi {
		padding: var(--multi-select-padding, var(--internal-padding));
	}

	.multi input {
		padding: var(--multi-select-input-padding, 0);
		position: relative;
		margin: var(--multi-select-input-margin, 5px 0);
		flex: 1 1 40px;
	}

	.svelte-select.error {
		border: var(--error-border, 1px solid #ff2d55);
		background: var(--error-background, #fff);
	}

	.a11y-text {
		z-index: 9999;
		border: 0px;
		clip: rect(1px, 1px, 1px, 1px);
		height: 1px;
		width: 1px;
		position: absolute;
		overflow: hidden;
		padding: 0px;
		white-space: nowrap;
	}

	.multi-item {
		background: var(--multi-item-bg, #ebedef);
		margin: var(--multi-item-margin, 0);
		outline: var(--multi-item-outline, 1px solid #ddd);
		border-radius: var(--multi-item-border-radius, 4px);
		height: var(--multi-item-height, 25px);
		line-height: var(--multi-item-height, 25px);
		display: flex;
		cursor: default;
		padding: var(--multi-item-padding, 0 5px);
		overflow: hidden;
		gap: var(--multi-item-gap, 4px);
		outline-offset: -1px;
		max-width: var(--multi-max-width, none);
		color: var(--multi-item-color, var(--item-color));
	}

	.multi-item.disabled:hover {
		background: var(--multi-item-disabled-hover-bg, #ebedef);
		color: var(--multi-item-disabled-hover-color, #c1c6cc);
	}

	.multi-item-text {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.multi-item-clear {
		display: flex;
		align-items: center;
		justify-content: center;
		--clear-icon-color: var(--multi-item-clear-icon-color, #000);
	}

	.multi-item.active {
		outline: var(--multi-item-active-outline, 1px solid #006fe8);
	}

	.svelte-select-list {
		box-shadow: var(--list-shadow, 0 2px 3px 0 rgba(44, 62, 80, 0.24));
		border-radius: var(--list-border-radius, 4px);
		max-height: var(--list-max-height, 252px);
		overflow-y: auto;
		background: var(--list-background, #fff);
		position: var(--list-position, absolute);
		z-index: var(--list-z-index, 5000);
		border: var(--list-border);
		font-size: small !important;
	}

	.prefloat {
		opacity: 0;
		pointer-events: none;
	}

	.list-group-title {
		color: var(--group-title-color, #8f8f8f);
		cursor: default;
		font-size: var(--group-title-font-size, 16px);
		font-weight: var(--group-title-font-weight, 600);
		height: var(--height, 42px);
		line-height: var(--height, 42px);
		padding: var(--group-title-padding, 0 20px);
		text-overflow: ellipsis;
		overflow-x: hidden;
		white-space: nowrap;
		text-transform: var(--group-title-text-transform, uppercase);
	}

	.empty {
		text-align: var(--list-empty-text-align, center);
		padding: var(--list-empty-padding, 20px 0);
		color: var(--list-empty-color, #78848f);
	}

	.item {
		cursor: default;
		height: var(--item-height, var(--height, 42px));
		line-height: var(--item-line-height, var(--height, 42px));
		padding: var(--item-padding, 0 20px);
		color: var(--item-color, inherit);
		text-overflow: ellipsis;
		overflow: hidden;
		white-space: nowrap;
		transition: var(--item-transition, all 0.2s);
		align-items: center;
		width: 100%;
	}

	.item.group-item {
		padding-left: var(--group-item-padding-left, 40px);
	}

	.item:active {
		background: var(--item-active-background, #b9daff);
	}

	.item.active {
		background: var(--item-is-active-bg, #007aff);
		color: var(--item-is-active-color, #fff);
	}

	.item.first {
		border-radius: var(--item-first-border-radius, 4px 4px 0 0);
	}

	.item.hover:not(.active) {
		background: var(--item-hover-bg, #e7f2ff);
		color: var(--item-hover-color, inherit);
	}

	.item.not-selectable,
	.item.hover.item.not-selectable,
	.item.active.item.not-selectable,
	.item.not-selectable:active {
		color: var(--item-is-not-selectable-color, #999);
		background: transparent;
	}

	.required {
		opacity: 0;
		z-index: -1;
		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
	}
</style>
