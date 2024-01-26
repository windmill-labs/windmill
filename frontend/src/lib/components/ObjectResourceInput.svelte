<script lang="ts">
	import ResourcePicker from './ResourcePicker.svelte'
	import S3ObjectPicker from './S3ObjectPicker.svelte'

	export let format: string
	export let value: any
	export let disablePortal = false
	export let showSchemaExplorer = false

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	let path: string = ''

	function resourceToValue() {
		if (path) {
			value = `$res:${path}`
		} else {
			value = undefined
		}
	}

	function isResource() {
		return isString(value) && value.length >= '$res:'.length
	}

	function valueToPath() {
		if (isResource()) {
			path = value.substr('$res:'.length)
		}
	}

	$: value && valueToPath()
</script>

<div class="flex flex-row w-full flex-wrap gap-x-2 gap-y-0.5">
	{#if format === 'resource-s3_object'}
		<S3ObjectPicker bind:value />
	{:else}
		<ResourcePicker
			{disablePortal}
			on:change={(e) => {
				path = e.detail
				resourceToValue()
			}}
			bind:value={path}
			resourceType={format.split('-').length > 1 ? format.substring('resource-'.length) : undefined}
			{showSchemaExplorer}
		/>
	{/if}
</div>
