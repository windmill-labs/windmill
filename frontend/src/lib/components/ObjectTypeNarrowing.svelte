<script lang="ts">
	import ResourceTypePicker from './ResourceTypePicker.svelte'

	export let format: string | undefined

	let resource: string | undefined = format?.startsWith('resource-')
		? format.substring('resource-'.length)
		: undefined
</script>

<div class="mt-2" />
<ResourceTypePicker
	on:click={(e) => {
		resource = e.detail
		format = resource != undefined ? `resource-${resource}` : undefined
	}}
	value={resource}
	nonePickable={true}
/>
