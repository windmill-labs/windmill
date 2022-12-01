<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'

	export let resource_type: string
	export let args: Record<string, any> = {}
	export let password: string

	let schema = emptySchema()
	async function loadSchema() {
		const rt = await ResourceService.getResourceType({
			workspace: $workspaceStore!,
			path: resource_type
		})
		schema = rt.schema
	}
	$: {
		$workspaceStore && loadSchema()
	}
</script>

<SchemaForm {password} isValid {schema} bind:args />
