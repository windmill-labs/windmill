<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import ResourceTypePicker from './ResourceTypePicker.svelte'

	export let format: string | undefined

	let resource: string | undefined = format?.startsWith('resource-')
		? format.substring('resource-'.length)
		: undefined
	const dispatch = createEventDispatcher()
</script>

<ResourceTypePicker
	on:click={(e) => {
		resource = e.detail
		format = resource != undefined ? `resource-${resource}` : undefined
		dispatch('change', format)
	}}
	value={resource}
	nonePickable={true}
/>
