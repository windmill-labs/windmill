<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Select from './apps/svelte-select/lib/index'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'

	import DarkModeObserver from './DarkModeObserver.svelte'

	const dispatch = createEventDispatcher()

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let resourceType: string | undefined = undefined
	export let disablePortal = false

	let valueSelect =
		initialValue || value
			? {
					value: value ?? initialValue,
					label: value ?? initialValue
			  }
			: undefined

	let collection = [valueSelect]

	async function loadResources(resourceType: string | undefined) {
		const nc = (
			await ResourceService.listResource({
				workspace: $workspaceStore!,
				resourceType
			})
		).map((x) => ({
			value: x.path,
			label: x.path
		}))

		// TODO check if this is needed
		if (!nc.find((x) => x.value == value) && (initialValue || value)) {
			nc.push({ value: value ?? initialValue!, label: value ?? initialValue! })
		}
		collection = nc
	}

	$: {
		if ($workspaceStore) {
			loadResources(resourceType)
		}
	}
	$: dispatch('change', value)

	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<Select
	portal={!disablePortal}
	value={valueSelect}
	on:change={(e) => {
		value = e.detail.value
		valueSelect = e.detail
	}}
	on:clear={() => {
		value = undefined
		valueSelect = undefined
	}}
	items={collection}
	class="text-clip grow min-w-0"
	placeholder="{resourceType ?? 'any'} resource"
	inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
	containerStyles={darkMode
		? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
		: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
/>
