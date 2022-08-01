<script lang="ts">
	import ResourceTypePicker from './ResourceTypePicker.svelte'

	export let format: string | undefined

	let resource: string | undefined = format?.startsWith('resource-')
		? format.substring('resource-'.length)
		: undefined

	$: format = resource != undefined ? `resource-${resource}` : undefined
</script>

<div class="mt-2" />
<ResourceTypePicker bind:value={resource} nonePickable={true} />
