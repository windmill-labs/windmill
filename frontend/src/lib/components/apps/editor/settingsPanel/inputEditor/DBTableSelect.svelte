<script lang="ts">
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import type { StaticInput, StaticOptions } from '../../../inputType'
	import Select from '$lib/components/apps/svelte-select/lib/index'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'

	export let componentInput: StaticInput<any> | undefined
	export let selectOptions: StaticOptions['selectOptions'] | undefined = undefined
	export let id: string | undefined

	let selectedValue: string | undefined = componentInput?.value ?? undefined
	let darkMode: boolean = false

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')
</script>

<DarkModeObserver bind:darkMode />

{#if selectOptions && componentInput}
	<Select
		portal={false}
		value={selectedValue}
		on:change={(e) => {
			selectedValue = e.detail.value

			if (id) {
				$componentControl?.[id]?.reset?.(e.detail.value)
			}
		}}
		on:clear={() => {
			if (id) {
				$componentControl?.[id]?.reset?.(undefined)
			}
		}}
		items={selectOptions}
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
