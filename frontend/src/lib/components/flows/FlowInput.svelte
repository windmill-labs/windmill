<script lang="ts">
	import { emptySchema } from '$lib/utils'

	import SchemaEditor from '../SchemaEditor.svelte'
	import { flowStore } from './../flows/flowStore'
	import CopyFirstStepSchema from './CopyFirstStepSchema.svelte'
	import FlowBox from './FlowBox.svelte'
</script>

<FlowBox title="Flow Inputs">
	<div slot="header">
		<CopyFirstStepSchema />
	</div>
	<div slot="content">
		<div class="p-6 border-t border-gray-300">
			<SchemaEditor
				on:change={() => {
					$flowStore = $flowStore
				}}
				schema={$flowStore?.schema ?? emptySchema()}
			/>
		</div>
	</div>
</FlowBox>
