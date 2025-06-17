<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { createFloatingActions } from 'svelte-floating-ui'
	import { tick } from 'svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import MultiselectLegacy from '$lib/components/multiselect/MultiSelectLegacy.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import { deepEqual } from 'fast-equals'

	let {
		items,
		value = $bindable(),
		placeholder = undefined,
		target = undefined,
		topPlacement = false,
		allowUserOptions = undefined
	} = $props<{
		items: any[]
		value?: string[]
		placeholder?: string
		target?: string | HTMLElement
		topPlacement?: boolean
		allowUserOptions?: boolean | 'append'
	}>()

	$effect.pre(() => {
		if (value === undefined) value = []
	})

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		placement: topPlacement ? 'top-start' : 'bottom-start',
		middleware: [offset(5), flip(), shift()]
	})

	let outerDiv = $state<HTMLDivElement | undefined>(undefined)
	let portalRef = $state<HTMLDivElement | undefined>(undefined)
	let darkMode = $state(false)
	let w = $state(0)
	let open = $state(false)
	function moveOptionsToPortal() {
		// Find ul element with class 'options' within the outerDiv
		const ul = outerDiv?.querySelector('.options')
		if (ul) {
			// Move the ul element to the portal
			portalRef?.appendChild(ul)
		}
	}

	$effect(() => {
		if (portalRef && outerDiv && (allowUserOptions || items?.length > 0)) {
			tick().then(() => {
				moveOptionsToPortal()
			})
		}
	})
</script>

<DarkModeObserver bind:darkMode />

<div use:floatingRef bind:clientWidth={w}>
	{#if !value || Array.isArray(value)}
		<div class="border rounded-md border-gray-300 shadow-sm dark:border-gray-600 !w-full">
			<MultiselectLegacy
				{allowUserOptions}
				outerDivClass={`!text-xs`}
				ulSelectedClass="overflow-auto"
				bind:outerDiv
				--sms-border={'none'}
				--sms-min-height={'30px'}
				--sms-focus-border={'none'}
				--sms-selected-bg={darkMode ? '#c7d2fe' : '#e0e7ff'}
				--sms-selected-text-color={darkMode ? '#312e81' : '#3730a3'}
				bind:selected={
					() => [...value],
					(newVal) => {
						if (!deepEqual(value, newVal)) {
							value = newVal
						}
					}
				}
				{placeholder}
				options={items}
				on:close={() => {
					open = false
				}}
				on:open={() => {
					open = true
				}}
				let:option
				disableRemoveAll
			>
				<div
					class="w-full text-sm"
					role="option"
					tabindex="0"
					onmouseup={(e) => {
						e.stopPropagation()
					}}
					onpointerdown={(e) => {
						e.stopPropagation()
						let newe = new MouseEvent('mouseup')
						e.target?.['parentElement']?.dispatchEvent(newe)
					}}
					aria-selected={value?.includes(option)}
				>
					{option}
				</div>
			</MultiselectLegacy>
		</div>
		<Portal {target} name="multi-select">
			<div use:floatingContent class="z5000" hidden={!open}>
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					bind:this={portalRef}
					class="multiselect"
					style={`min-width: ${w}px;`}
					onclick={(e) => {
						e.stopPropagation()
					}}
					role="listbox"
					tabindex="0"
				></div>
			</div>
		</Portal>
	{:else}
		Value {value} is not an array
	{/if}
</div>
