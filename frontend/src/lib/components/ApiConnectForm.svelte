<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'

	export let resource_type: string
	export let args: Record<string, any> | any = {}
	export let password: string
	export let isValid = true

	let schema = emptySchema()
	let notFound = false
	async function loadSchema() {
		try {
			const rt = await ResourceService.getResourceType({
				workspace: $workspaceStore!,
				path: resource_type
			})
			schema = rt.schema
			notFound = false
		} catch (e) {
			rawCode = '{}'

			notFound = true
		}
	}
	$: {
		$workspaceStore && loadSchema()
	}
	$: notFound && rawCode && parseJson()

	function parseJson() {
		try {
			args = JSON.parse(rawCode)
			error = ''
			isValid = true
		} catch (e) {
			isValid = false
			error = e.message
		}
	}
	let error = ''
	let rawCode = ''
</script>

{#if notFound}
	<p class="italic text-gray-500 text-xs mb-4"
		>No corresponding resource type found in your workspace for {resource_type}. Define the value in
		JSON directly</p
	>
	{#if !emptyString(error)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
			>{error}</span
		>{:else}<div class="py-2" />{/if}
	<SimpleEditor autoHeight lang="json" bind:code={rawCode} fixedOverflowWidgets={false} />
{:else}
	<SchemaForm {password} isValid {schema} bind:args />
{/if}
