<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { Flow } from '$lib/gen'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { flowStateStore, flowStateToFlow } from './flows/flowState'
	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import RunForm from './RunForm.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import Tabs from './common/tabs/Tabs.svelte'

	export let i: number
	export let flow: Flow
	export let schema: Schema

	export let args: Record<string, any> = {}

	let stepArgs: Record<string, any> = {}

	let tab: 'upto' | 'justthis' = 'upto'
	let viewPreview = false

	let uptoText =
		i >= flow.value.modules.length - 1 ? 'Preview whole flow' : 'Preview up to this step'
	let jobId: string

	export async function runPreview(args: any) {
		viewPreview = true
		flow = flowStateToFlow($flowStateStore, flow)

		let newFlow: Flow =
			tab == 'upto' ? truncateFlow(flow) : setInputTransformFromArgs(extractStep(flow), args)
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
		let input_transform = {}
		Object.entries(args).forEach(([key, value]) => {
			input_transform[key] = {
				type: 'static',
				value: value
			}
		})
		flow.value.modules[0].input_transform = input_transform
		return flow
	}
</script>

<button
	class="w-full rounded border-1 border border-gray-200"
	on:click={() => {
		viewPreview = !viewPreview
	}}
>
	<h2 class="flex justify-center text-gray-600">
		<div>
			Preview
			<Icon class="ml-1" data={viewPreview ? faChevronUp : faChevronDown} scale={1} />
		</div>
	</h2>
</button>

{#if viewPreview}
	{#if i != flow.value.modules.length}
		<Tabs bind:selected={tab}>
			<Tab value="upto">{uptoText}</Tab>
			<Tab value="justthis">Preview just this step</Tab>
			<svelte:fragment slot="content">
				<TabContent value="upto">
					<RunForm
						runnable={truncateFlow(flow)}
						runAction={(_, args) => runPreview(args)}
						schedulable={false}
						buttonText={uptoText}
						detailed={false}
						bind:args
					/>
				</TabContent>
				<TabContent value="justthis">
					<RunForm
						runnable={extractStep(flow)}
						runAction={(_, args) => runPreview(args)}
						schedulable={false}
						buttonText="Preview just this step"
						detailed={false}
						args={stepArgs}
					/>
				</TabContent>
			</svelte:fragment>
		</Tabs>
	{/if}

	{#if jobId}
		<div class="w-full flex justify-center">
			<FlowStatusViewer
				{jobId}
				on:jobsLoaded={(e) => mapJobResultsToFlowState(e.detail, tab, i)}
				root={true}
			/>
		</div>
	{/if}
{/if}
