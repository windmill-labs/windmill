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
<FlowCard title="Flow Inputs">
	<div slot="header">
		<div class="flex flex-row space-x-4">
			<Button
				color="light"
				size="sm"
				on:click={() => {
					capturePayload.openDrawer()
				}}
				variant="border"
			>
				Capture from a request to seed inputs
			</Button>
			<Button
				color="light"
				size="sm"
				disabled={$flowStore.value.modules.length === 0 ||
					isEmptyFlowModule($flowStore.value.modules[0])}
				on:click={copyFirstStepSchema}
				variant="border"
			>
				Copy from first step schema {#if $flowStore.value.modules.length === 0 || isEmptyFlowModule($flowStore.value.modules[0])}
					(no steps to copy from!){/if}
			</Button>
		</div>
	</div>
	<div>
		<div class="p-6">
			<SchemaEditor
				bind:schema={$flowStore.schema}
				on:change={() => {
					$flowStore = $flowStore
				}}
			/>
		</div>
	</div>
	<div class="p-6">
		<SchemaForm bind:schema={$flowStore.schema} editableSchema={true} />
	</div>
</FlowCard>
