<script lang="ts">
	import { ResourceService } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import ResourcePicker from './ResourcePicker.svelte'

	export let format: string
	export let value: any

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String
	}

	let path: string = ''
	let args: Record<string, any> = {}

	let schema: any | undefined = undefined
	let resourceTypeName: string = ''

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
		}
	}

	$: value && valueToPath()
</script>

<div class="flex flex-row w-full flex-wrap gap-x-2 gap-y-0.5">
	<ResourcePicker
		on:refresh={() => loadSchema(format)}
		on:change={(e) => {
			path = e.detail
			resourceToValue()
		}}
		bind:value={path}
		resourceType={format.split('-').length > 1 ? format.substring('resource-'.length) : undefined}
	/>
</div>
