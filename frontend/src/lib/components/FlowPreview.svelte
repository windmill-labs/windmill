<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { Flow } from '$lib/gen'
	import { sendUserToast, truncateRev } from '$lib/utils'

	import { flowStateStore, flowStateToFlow } from './flows/flowState'
	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import RunForm from './RunForm.svelte'

	export let indexes: string
	export let flow: Flow
	export let schema: Schema

	const [i, j] = indexes.split('-').map(Number)

	let stepArgs: Record<string, any> = {}
	let jobId: string

	export async function runPreview(args: any) {
		flow = flowStateToFlow($flowStateStore, flow)

		let newFlow: Flow = setInputTransformFromArgs(extractStep(flow), args)
		jobId = await runFlowPreview(args, newFlow)

		sendUserToast(`started preview ${truncateRev(jobId, 10)}`)
	}

	function extractStep(flow: Flow): Flow {
		const localFlow = JSON.parse(JSON.stringify(flow))
		const mod = flow.value.modules[i].value
		console.log(mod, j)
		if (j != undefined && mod.type === 'forloopflow') {
			localFlow.value.modules = mod.modules.slice(j, j + 1)
		} else {
			localFlow.value.modules = flow.value.modules.slice(i, i + 1)
		}
		console.log(localFlow)
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
			on:jobsLoaded={(e) => mapJobResultsToFlowState(e.detail, 'justthis', i, j)}
			root={true}
		/>
	</div>
{/if}
