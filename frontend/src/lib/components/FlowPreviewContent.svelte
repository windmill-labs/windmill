<script lang="ts">
	import { faClose, faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import { flowStateStore, flowStateToFlow } from './flows/flowState'
	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { flowStore } from './flows/flowStore'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import SchemaForm from './SchemaForm.svelte'

	export let args: Record<string, any> = {}

	let jobId: string | undefined = undefined
	let isValid: boolean = false
	let intervalState: 'idle' | 'canceled' | 'done' | 'running' = 'idle'

	$: newFlow = flowStateToFlow($flowStateStore, $flowStore)
	$: steps = newFlow?.value.modules.length ?? 0

	const dispatch = createEventDispatcher()

	export async function runPreview(args: Record<string, any>) {
		jobId = await runFlowPreview(args, newFlow)

		intervalState = 'running'
	}

	onDestroy(() => {
		intervalState = 'done'
	})
</script>

<div class="flex divide-y flex-col space-y-2 h-screen bg-white p-6 w-full">
	<div class="flex justify-between">
		<div class="flex flex-row justify-center items-center">
			<div class="flex justify-center p-2 w-8 h-8 bg-blue-200 rounded-lg mr-2">
				<Icon data={faPlay} scale={1} class="text-blue-500" />
			</div>

			<h3 class="text-lg leading-6 font-bold text-gray-900">Flow preview</h3>
		</div>
		<Button color="alternative" on:click={() => dispatch('close')}>
			<Icon data={faClose} />
		</Button>
	</div>
	<div class="max-h-80 overflow-y-auto">
		<SchemaForm schema={$flowStore.schema} bind:isValid bind:args />
	</div>
	{#if intervalState === 'running'}
		<Button
			disabled={!isValid}
			color="red"
			on:click={() => {
				intervalState = 'canceled'
				jobId = undefined
			}}
			size="md"
		>
			Cancel
		</Button>
	{:else}
		<Button disabled={!isValid} class="blue-button" on:click={() => runPreview(args)} size="md">
			{`Run${intervalState === 'done' ? ' again' : ''}`}
		</Button>
	{/if}

	<div class="h-full overflow-y-auto mb-16 grow">
		{#if jobId}
			<FlowStatusViewer
				{jobId}
				on:jobsLoaded={(e) => mapJobResultsToFlowState(e.detail, 'upto', steps - 1)}
				root={true}
			/>
		{/if}
	</div>
</div>
