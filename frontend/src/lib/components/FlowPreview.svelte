<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { Flow } from '$lib/gen'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { HSplitPane } from 'svelte-split-pane'

	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { flowStore } from './flows/flowStore'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import RunForm from './RunForm.svelte'

	export let indexes: string
	export let schema: Schema

	const [i, j] = indexes.split('-').filter(Boolean).map(Number)

	let stepArgs: Record<string, any> = {}
	let jobId: string

	export async function runPreview(args: any) {
		let newFlow: Flow = setInputTransformFromArgs(extractStep($flowStore), args)
		jobId = await runFlowPreview(args, newFlow)

		sendUserToast(`started preview ${truncateRev(jobId, 10)}`)
	}

	function extractStep(flow: Flow): Flow {
		const localFlow = JSON.parse(JSON.stringify(flow))
		const mod = flow.value.modules[i].value
		if (j != undefined && mod.type === 'forloopflow') {
			localFlow.value.modules = mod.modules.slice(j, j + 1)
		} else {
			localFlow.value.modules = flow.value.modules.slice(i, i + 1)
		}
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

<HSplitPane leftPaneSize="50%" rightPaneSize="50%" minLeftPaneSize="20%" minRightPaneSize="20%">
	<left slot="left" class="relative">
		<div class="overflow-auto h-full p-4">
			<RunForm
				runnable={extractStep($flowStore)}
				runAction={(_, args) => runPreview(args)}
				schedulable={false}
				buttonText="Test just this step"
				detailed={false}
				args={stepArgs}
			/>
		</div>
	</left>
	<right slot="right">
		<div class="overflow-auto h-full p-4">
			{#if jobId}
				<FlowStatusViewer
					{jobId}
					on:jobsLoaded={(e) => mapJobResultsToFlowState(e.detail, 'justthis', i, j)}
				/>
			{:else}
				<span class="text-sm">No results yet</span>
			{/if}
		</div>
	</right>
</HSplitPane>
