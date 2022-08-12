<script lang="ts">
	import SchemaEditor from '../SchemaEditor.svelte'
	import { flowStore } from './../flows/flowStore'
	import CopyFirstStepSchema from './CopyFirstStepSchema.svelte'
	import FlowBox from './FlowBox.svelte'
</script>

<FlowBox title="Flow Inputs">
	<div slot="header"><CopyFirstStepSchema /></div>
	<div slot="content">
		<SchemaEditor
			on:change={() => {
				$flowStore = $flowStore
			}}
			schema={$flowStore.schema}
		/>
	</div>
</FlowBox>
