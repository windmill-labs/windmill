<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import S3ObjectPicker from './S3ObjectPicker.svelte'
	import type SimpleEditor from './SimpleEditor.svelte'

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	interface Props {
		format: string
		value: any
		disablePortal?: boolean
		showSchemaExplorer?: boolean
		selectFirst?: boolean
		defaultValue: any
		editor?: SimpleEditor | undefined
		path?: string
	}

	let {
		format,
		value = $bindable(),
		disablePortal = false,
		showSchemaExplorer = false,
		selectFirst = false,
		defaultValue,
		editor = $bindable(undefined)
	}: Props = $props()

	function isResource() {
		return isString(value) && value.length >= '$res:'.length
	}

	function valueToPath() {
		if (isResource()) {
			return value.substr('$res:'.length)
		}
	}
</script>

<!-- {JSON.stringify({ value })} -->
<div class="flex flex-row w-full flex-wrap gap-x-2 gap-y-0.5">
	{#if format === 'resource-s3_object'}
		<S3ObjectPicker bind:value />
	{:else if value == undefined || typeof value === 'string'}
		<ResourcePicker
			{selectFirst}
			{disablePortal}
			on:clear
			bind:value={
				() => valueToPath(),
				(v) => {
					if (v == undefined) {
						value = undefined
					} else {
						value = `$res:${v}`
					}
				}
			}
			initialValue={typeof defaultValue == 'string' && defaultValue.startsWith('$res:')
				? defaultValue.substr('$res:'.length)
				: defaultValue}
			resourceType={format.split('-').length > 1 ? format.substring('resource-'.length) : undefined}
			{showSchemaExplorer}
		/>
	{:else}
		{#await import('$lib/components/JsonEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default bind:editor code={JSON.stringify(value, null, 2)} bind:value />
		{/await}
	{/if}
</div>
