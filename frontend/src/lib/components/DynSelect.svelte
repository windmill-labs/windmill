<script lang="ts">
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

	export let value: any = undefined

	let darkMode: boolean = false

	async function getItemsFromOptions(txt: string) {
		console.log('FOO')
		return [
			{ value: '1', label: 'One' },
			{ value: '2', label: 'Two' },
			{ value: '3', label: 'Three' }
		]
	}
</script>

<DarkModeObserver bind:darkMode />

<Select
	bind:value
	computeOnClick
	loadOptions={getItemsFromOptions}
	inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
	containerStyles={darkMode
		? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
		: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
/>
