<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { Flow } from '$lib/gen'
	import { sendUserToast, truncateRev } from '$lib/utils'

	import { flowStateStore, flowStateToFlow } from './flows/flowState'
	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import RunForm from './RunForm.svelte'

	export let i: number
	export let flow: Flow
	export let schema: Schema

	let stepArgs: Record<string, any> = {}
	let jobId: string

	export async function runPreview(args: any) {
		flow = flowStateToFlow($flowStateStore, flow)

		let newFlow: Flow = setInputTransformFromArgs(extractStep(flow), args)
		jobId = await runFlowPreview(args, newFlow)

		sendUserToast(`started preview ${truncateRev(jobId, 10)}`)
	}

	function truncateFlow(flow: Flow): Flow {
		const localFlow = JSON.parse(JSON.stringify(flow))
		localFlow.value.modules = flow.value.modules.slice(0, i + 1)
		return localFlow
	}

	function extractStep(flow: Flow): Flow {
		const localFlow = JSON.parse(JSON.stringify(flow))
		localFlow.value.modules = flow.value.modules.slice(i, i + 1)
		localFlow.schema = schema
		return localFlow
	}

	function setInputTransformFromArgs(flow: Flow, args: any) {
		let input_transforms = {}
		Object.entries(args).forEach(([key, value]) => {
			input_transforms[key] = {
				type: 'static',
				value: value
			}
		})
		flow.value.modules[0].input_transforms = input_transforms
		return flow
	}
</script>

<RunForm
	runnable={extractStep(flow)}
	runAction={(_, args) => runPreview(args)}
	schedulable={false}
	buttonText="Test just this step"
	detailed={false}
	args={stepArgs}
/>

{#if jobId}
	<div class="w-full flex justify-center">
		<FlowStatusViewer
			{jobId}
			on:jobsLoaded={(e) => mapJobResultsToFlowState(e.detail, 'justthis', i)}
			root={true}
		/>
	</div>
{/if}
