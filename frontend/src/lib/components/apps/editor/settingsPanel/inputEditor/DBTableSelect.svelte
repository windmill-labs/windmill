<script lang="ts">
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import type { StaticInput, StaticOptions } from '../../../inputType'
	import Select from '$lib/components/apps/svelte-select/lib/index'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'

	export let componentInput: StaticInput<any> | undefined
	export let selectOptions: StaticOptions['selectOptions'] | undefined = undefined
	export let id: string | undefined

	let selectedValue: string | undefined = componentInput?.value ?? undefined
	let darkMode: boolean = false

	$: sortedOptions = Array.isArray(selectOptions)
		? selectOptions?.sort((a, b) => a.localeCompare(b))
		: []
</script>

<DarkModeObserver bind:darkMode />

{#if selectOptions && componentInput}
	<Select
		portal={false}
		value={selectedValue}
		on:change={(e) => {
			selectedValue = e.detail.value
			if (componentInput?.type === 'static') {
				componentInput.value = e.detail.value
			}
		}}
		on:clear={() => {
			if (componentInput?.type === 'static') {
				componentInput.value = undefined
			}
		}}
		items={sortedOptions}
		class="text-clip grow min-w-0"
		placeholder="Select a table"
		inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
		containerStyles={darkMode
			? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
			: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		clearable
	/>
{:else}
	<input {id} class="w-full p-2 border rounded-md" type="text" />
{/if}
