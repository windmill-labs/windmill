<script lang="ts">
	// @ts-ignore
	import Portal from '$lib/components/Portal.svelte'

	import { createFloatingActions } from 'svelte-floating-ui'
	import { tick } from 'svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import MultiSelect from '$lib/components/multiselect/MultiSelect.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'

	export let items: any[]
	let propValue: string[] | undefined = []
	export { propValue as value }
	$: value = propValue || []
	export let placeholder: string | undefined = undefined
	export let target: string | HTMLElement | undefined = undefined
	export let topPlacement = false
	export let allowUserOptions: boolean | 'append' | undefined = undefined
	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		placement: topPlacement ? 'top-start' : 'bottom-start',
		middleware: [offset(5), flip(), shift()]
	})

	let outerDiv: HTMLDivElement | undefined = undefined
	let portalRef: HTMLDivElement | undefined = undefined

	function moveOptionsToPortal() {
		// Find ul element with class 'options' within the outerDiv
		const ul = outerDiv?.querySelector('.options')
		if (ul) {
			// Move the ul element to the portal
			portalRef?.appendChild(ul)
		}
	}

	$: if (portalRef && outerDiv && (allowUserOptions || items?.length > 0)) {
		tick().then(() => {
			moveOptionsToPortal()
		})
	}

	// bg-indigo-100 text-indigo-800 dark:bg-indigo-200 dark:text-indigo-900
	let darkMode: boolean = false

	let w = 0
	let open: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<div use:floatingRef bind:clientWidth={w}>
	{#if !value || Array.isArray(value)}
		<div class="border rounded-md border-gray-300 shadow-sm dark:border-gray-600 !w-full">
			<MultiSelect
				{allowUserOptions}
				outerDivClass={`!text-xs`}
				ulSelectedClass="overflow-auto"
				bind:outerDiv
				--sms-border={'none'}
				--sms-min-height={'30px'}
				--sms-focus-border={'none'}
				--sms-selected-bg={darkMode ? '#c7d2fe' : '#e0e7ff'}
				--sms-selected-text-color={darkMode ? '#312e81' : '#3730a3'}
				bind:selected={value}
				on:change={() => {
					propValue = value
				}}
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
				<!-- needed because portal doesn't work for mouseup event en mobile -->
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div
					class="w-full text-sm"
					on:mouseup|stopPropagation
					on:pointerdown|stopPropagation={(e) => {
						let newe = new MouseEvent('mouseup')
						e.target?.['parentElement']?.dispatchEvent(newe)
					}}
				>
					{option}
				</div>
			</MultiSelect>
		</div>
		<Portal {target} name="multi-select">
			<div use:floatingContent class="z5000" hidden={!open}>
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div
					bind:this={portalRef}
					class="multiselect"
					style={`min-width: ${w}px;`}
					on:click|stopPropagation
				></div>
			</div>
		</Portal>
	{:else}
		Value {value} is not an array
	{/if}
</div>
