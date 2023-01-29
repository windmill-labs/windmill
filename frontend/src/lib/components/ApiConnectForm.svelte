<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'

	export let resource_type: string
	export let args: Record<string, any> | any = {}
	export let password: string
	export let isValid = true

	let schema = emptySchema()
	let notFound = false

	async function loadSchema() {
		rawCode = '{}'
		viewJsonSchema = false
		try {
			const rt = await ResourceService.getResourceType({
				workspace: $workspaceStore!,
				path: resource_type
			})
			schema = rt.schema
			notFound = false
		} catch (e) {
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
	let viewJsonSchema = false

	function switchTab(asJson: boolean) {
		viewJsonSchema = asJson
		if (asJson) {
			rawCode = JSON.stringify(args, null, 2)
		} else {
			parseJson()
		}
	}
</script>

{#if !notFound}
	<div class="w-full flex flex-row-reverse">
		<Toggle
			on:change={(e) => switchTab(e.detail)}
			options={{
				right: 'As JSON'
			}}
		/>
	</div>
{:else}
	<p class="italic text-gray-500 text-xs mb-4"
		>No corresponding resource type found in your workspace for {resource_type}. Define the value in
		JSON directly</p
	>
{/if}
{#if notFound || viewJsonSchema}
	{#if !emptyString(error)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
			>{error}</span
		>{:else}<div class="py-2" />{/if}
	<SimpleEditor autoHeight lang="json" bind:code={rawCode} fixedOverflowWidgets={false} />
{:else}
	<SchemaForm noDelete {password} isValid {schema} bind:args />
{/if}
