<script lang="ts">
	import ResourceTypePicker from './ResourceTypePicker.svelte'

	interface Props {
		format: string | undefined
	}

	let { format = $bindable() }: Props = $props()

	let resource: string | undefined = $state(
		format?.startsWith('resource-') ? format.substring('resource-'.length) : undefined
	)
</script>

<ResourceTypePicker
	on:click={(e) => {
		resource = e.detail
		format = `resource-${resource ?? ''}`
	}}
	value={resource}
	nonePickable={true}
/>
