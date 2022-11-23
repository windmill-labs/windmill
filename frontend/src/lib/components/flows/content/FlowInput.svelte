<script lang="ts">
	import { Button } from '$lib/components/common'

	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import { copyFirstStepSchema, flowStore } from '../flowStore'
	import { isEmptyFlowModule } from '../utils'
	import CapturePayload from './CapturePayload.svelte'

	let capturePayload: CapturePayload
</script>

<CapturePayload bind:this={capturePayload} />
<FlowCard title="Flow Input">
	<div class="p-6">
		<div class="flex flex-row items-center space-x-4 pb-2 border-b border-gray-400">
			<div>Copy input's schema from</div>
			<Button
				color="dark"
				size="sm"
				on:click={() => {
					capturePayload.openDrawer()
				}}
			>
				A request
			</Button>
			<Button
				color="dark"
				size="sm"
				disabled={$flowStore.value.modules.length === 0 ||
					isEmptyFlowModule($flowStore.value.modules[0])}
				on:click={copyFirstStepSchema}
			>
				First step's inputs
			</Button>
		</div>
		<div class="pt-6">
			<SchemaEditor
				bind:schema={$flowStore.schema}
				on:change={() => {
					$flowStore = $flowStore
				}}
			/>
		</div>
	</div>
	<div class="p-6">
		<h2 class="mb-4">Customize Inputs</h2>
		<SchemaForm bind:schema={$flowStore.schema} editableSchema={true} />
	</div>
</FlowCard>
