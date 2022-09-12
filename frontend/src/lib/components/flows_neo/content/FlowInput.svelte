<script lang="ts">
	import { flowStore } from '$lib/components/flows/flowStore'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
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
				schema={$flowStore?.schema ?? emptySchema()}
			/>
		</div>
	</div>
</FlowCard>
