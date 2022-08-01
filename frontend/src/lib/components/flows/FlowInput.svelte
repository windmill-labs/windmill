<script lang="ts">
	import SchemaEditor from '../SchemaEditor.svelte'
	import { flowStore } from './../flows/flowStore'
	import CopyFirstStepSchema from './CopyFirstStepSchema.svelte'
	import FlowBox from './FlowBox.svelte'
	import FlowBoxHeader from './FlowBoxHeader.svelte'
</script>

<FlowBox>
	<FlowBoxHeader title="Flow Input">
		<CopyFirstStepSchema />
	</FlowBoxHeader>
	<div class="p-6">
		<SchemaEditor
			on:change={() => {
				$flowStore = $flowStore
			}}
			schema={$flowStore.schema}
		/>
	</div>
</FlowBox>
