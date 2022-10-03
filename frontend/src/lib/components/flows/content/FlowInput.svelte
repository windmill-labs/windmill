<script lang="ts">
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { emptySchema } from '$lib/utils'
	import FlowCard from '../common/FlowCard.svelte'

	import CopyFirstStepSchema from './CopyFirstStepSchema.svelte'
</script>

<FlowCard title="Flow Inputs">
	<div slot="header">
		<CopyFirstStepSchema />
	</div>
	<div>
		<div class="p-6">
			<SchemaEditor
				on:change={() => {
					$flowStore = $flowStore
				}}
				bind:schema={$flowStore.schema}
			/>
		</div>
	</div>
	<div class="p-6">
		<SchemaForm bind:schema={$flowStore.schema} editableSchema={true} />
	</div>
</FlowCard>
