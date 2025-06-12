<script lang="ts">
	import EditableSchemaForm from '$lib/components/EditableSchemaForm.svelte'
	import AddPropertyV2 from '$lib/components/schema/AddPropertyV2.svelte'
	import { Plus } from 'lucide-svelte'

	// import { initializeMode } from 'monaco-graphql/esm/initializeMode.js'
	// initializeMode()
	let schema = $state({
		type: 'object',
		properties: {
			name: { type: 'string' }
		}
	})

	let addPropertyV2: AddPropertyV2 | undefined = $state(undefined)
</script>

<AddPropertyV2 bind:this={addPropertyV2} bind:schema>
	{#snippet trigger()}
		<div
			class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
			id="add-flow-input-btn"
		>
			<Plus size={14} />
		</div>
	{/snippet}
</AddPropertyV2>
<EditableSchemaForm
	isFlowInput
	on:edit={(e) => {
		addPropertyV2?.openDrawer(e.detail)
	}}
	on:delete={(e) => {
		addPropertyV2?.handleDeleteArgument([e.detail])
	}}
	bind:schema
	editTab="inputEditor"
/>

<pre class="text-xs">{JSON.stringify(schema, null, 2)}</pre>
