<script lang="ts">
	import { ResourceService } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import RadioButton from './RadioButton.svelte'
	import ResourcePicker from './ResourcePicker.svelte'
	import SchemaForm from './SchemaForm.svelte'

	export let format: string
	export let value: any

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	let path: string = ''
	let args: Record<string, any> = {}

	if (!isString(value) && value) {
		args = value
	}

	valueToPath()

	let schema: any | undefined = undefined
	let isValid = true
	let resourceTypeName: string = ''

	let option: 'resource' | 'raw' = isString(value) || value == undefined ? 'resource' : 'raw'

	$: format.startsWith('resource-') && loadSchema(format)

	async function loadSchema(format: string) {
		resourceTypeName = format.substring('resource-'.length)
		schema = (
			await ResourceService.getResourceType({ workspace: $workspaceStore!, path: resourceTypeName })
		).schema
	}

	function argToValue() {
		if (option == 'resource') {
			value = `$res:${path}`
		} else {
			value = args
		}
	}
	function valueToPath() {
		path =
			isString(value) && value.length >= '$res:'.length ? value.substr('$res:'.length) : undefined
	}

	$: value && valueToPath()
</script>

<div class="flex flex-row w-full gap-2">
	<div class="shrink w-40">
		<RadioButton
			options={[
				[`Resource (${resourceTypeName})`, 'resource'],
				[`Raw object value`, 'raw']
			]}
			on:change={argToValue}
			bind:value={option}
		/>
	</div>
	<div class="grow">
		{#if option == 'resource'}
			<ResourcePicker
				on:refresh={() => loadSchema(format)}
				on:change={(e) => {
					path = e.detail
					argToValue()
				}}
				bind:value={path}
				resourceType={format.split('-').length > 1
					? format.substring('resource-'.length)
					: undefined}
			/>
		{:else}
			<div class="border rounded p-5 w-full">
				<h2 class="mb-5">
					Object of <a target="_blank" href="/resources">resource type</a>
					{resourceTypeName}
				</h2>
				{#if !isString(args)}
					<SchemaForm {schema} bind:isValid bind:args />
				{/if}
			</div>
		{/if}
	</div>
</div>
