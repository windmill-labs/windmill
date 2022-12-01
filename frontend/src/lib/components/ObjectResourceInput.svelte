<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { logout } from '$lib/logout'

	import { workspaceStore } from '$lib/stores'
	import { ToggleButton, ToggleButtonGroup } from './common'
	import ResourcePicker from './ResourcePicker.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'

	export let format: string
	export let value: any
	export let compact: boolean

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	let path: string = ''
	let args: Record<string, any> = {}

	let schema: any | undefined = undefined
	let isValid = true
	let resourceTypeName: string = ''

	let option: 'resource' | 'raw' = isString(value) || value == undefined ? 'resource' : 'raw'

	$: format.startsWith('resource-') && loadSchema(format)

	async function loadSchema(format: string) {
		resourceTypeName = format.substring('resource-'.length)
		try {
			schema = (
				await ResourceService.getResourceType({
					workspace: $workspaceStore!,
					path: resourceTypeName
				})
			).schema
		} catch (e) {
			schema = undefined
		}
	}

	function argToValue() {
		value = args
	}

	function resourceToValue() {
		value = `$res:${path}`
	}

	function isResource() {
		return isString(value) && value.length >= '$res:'.length
	}

	function valueToPath() {
		if (!isString(value) && value) {
			args = value
		}

		if (isResource()) {
			path = value.substr('$res:'.length)
		} else if (value !== undefined && value !== '') {
			option = 'raw'
		}
	}

	$: value && valueToPath()
</script>

<div class="flex flex-row w-full flex-wrap gap-x-2 gap-y-0.5">
	<ToggleButtonGroup
		col
		bind:selected={option}
		on:selected={(e) => {
			if (e.detail === 'resource') {
				resourceToValue()
			} else {
				argToValue()
			}
		}}
	>
		<ToggleButton light position="center" value="resource" size="xs">Resource</ToggleButton>
		<ToggleButton light position="center" value="raw" size="xs">Raw</ToggleButton>
	</ToggleButtonGroup>

	<div class="grow flex items-center">
		{#if option == 'resource'}
			<ResourcePicker
				on:refresh={() => loadSchema(format)}
				on:change={(e) => {
					path = e.detail
					resourceToValue()
				}}
				bind:value={path}
				resourceType={format.split('-').length > 1
					? format.substring('resource-'.length)
					: undefined}
			/>
		{:else}
			<div class="border rounded p-2 w-full">
				{#if schema != undefined}
					{#if !isString(args)}
						<SchemaForm {compact} {schema} bind:isValid bind:args />
					{/if}
				{:else}
					<SimpleEditor autoHeight lang="json" bind:value={args} />
				{/if}
			</div>
		{/if}
	</div>
</div>
