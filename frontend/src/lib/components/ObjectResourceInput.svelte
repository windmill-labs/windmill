<script lang="ts">
	import JsonEditor from './apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import S3ObjectPicker from './S3ObjectPicker.svelte'
	import SimpleEditor from './SimpleEditor.svelte'

	export let format: string
	export let value: any
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let selectFirst = false
	export let defaultValue: any
	export let editor: SimpleEditor | undefined = undefined
	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	export let path: string = ''

	function resourceToValue() {
		if (path && path != '') {
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
	{:else if value == undefined || typeof value === 'string'}
		<ResourcePicker
			{selectFirst}
			{disablePortal}
			on:change={(e) => {
				path = e.detail
				resourceToValue()
			}}
			on:clear
			bind:value={path}
			initialValue={typeof defaultValue == 'string' && defaultValue.startsWith('$res:')
				? defaultValue.substr('$res:'.length)
				: defaultValue}
			resourceType={format.split('-').length > 1 ? format.substring('resource-'.length) : undefined}
			{showSchemaExplorer}
		/>
	{:else}
		<JsonEditor bind:editor code={JSON.stringify(value, null, 2)} bind:value />
	{/if}
</div>
