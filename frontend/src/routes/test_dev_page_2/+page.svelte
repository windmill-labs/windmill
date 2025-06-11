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
</script>

<AddPropertyV2 bind:schema>
	{#snippet trigger()}
		<div
			class="w-full py-2 flex justify-center items-center border border-dashed rounded-md hover:bg-surface-hover"
			id="add-flow-input-btn"
		>
			<Plus size={14} />
		</div>
	{/snippet}
</AddPropertyV2>
<EditableSchemaForm isFlowInput bind:schema editTab="inputEditor" />

<pre class="text-xs">{JSON.stringify(schema, null, 2)}</pre>
