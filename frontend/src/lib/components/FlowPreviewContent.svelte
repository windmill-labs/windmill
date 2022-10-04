<script lang="ts">
	import { JobService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faClose, faPlay, faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { Button } from './common'
	import { createEventDispatcher, getContext, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import { flowStateStore } from './flows/flowState'
	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { flowStore } from './flows/flowStore'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview, selectedIdToIndexes } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'

	export let previewMode: 'upTo' | 'whole'

	let jobId: string | undefined = undefined
	let isValid: boolean = false
	let intervalState: 'idle' | 'canceled' | 'done' | 'running' = 'idle'

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	function extractFlow(previewMode: 'upTo' | 'whole'): Flow {
		if (previewMode === 'whole') {
			return $flowStore
		} else {
			const [parentIndex, childIndex] = selectedIdToIndexes($selectedId)

			const modules = $flowStore.value.modules.slice(0, Number(parentIndex) + 1)
			const flow = JSON.parse(JSON.stringify($flowStore))
			flow.value.modules = modules

			if (childIndex != undefined) {
				const lastModule = modules[modules.length - 1].value
				if (lastModule.type === 'forloopflow') {
					lastModule.modules = lastModule.modules.slice(0, Number(childIndex) + 1)
				} else {
					throw Error('Excepted forloopflow module')
				}
			}
			return flow
		}
	}

	const dispatch = createEventDispatcher()

	export async function runPreview(args: Record<string, any>) {
		const newFlow = extractFlow(previewMode)
		jobId = await runFlowPreview(args, newFlow)

		intervalState = 'running'
	}

	onDestroy(() => {
		intervalState = 'done'
	})
</script>

<div class="flex divide-y flex-col space-y-2 h-screen bg-white p-6 w-full">
	<div class="flex justify-between">
		<div class="flex flex-row justify-center items-center ">
			<div class="flex justify-center p-2 w-8 h-8 bg-blue-200 rounded-lg mr-2 ">
				<Icon data={faPlay} scale={1} class="text-blue-500" />
			</div>

			<h3 class="text-lg leading-6 font-bold text-gray-900">
				Test preview - {previewMode === 'upTo'
					? `up to step ${$selectedId.split('-').join(',')}`
					: ' whole flow'}
			</h3>
		</div>
		<Button
			variant="border"
			size="lg"
			color="dark"
			btnClasses="!p-0 !w-8 !h-8"
			on:click={() => {
				jobId = undefined
				intervalState = 'idle'
				dispatch('close')
			}}
		>
			<Icon data={faClose} />
		</Button>
	</div>
	<div class="grow pb-8 max-h-1/2 overflow-auto">
		<SchemaForm
			class="h-full pt-4"
			schema={$flowStore.schema}
			bind:isValid
			bind:args={$previewArgs}
		/>
	</div>
	{#if intervalState === 'running'}
		<Button
			disabled={!isValid}
			color="red"
			on:click={async () => {
				intervalState = 'canceled'
				try {
					jobId &&
						(await JobService.cancelQueuedJob({
							workspace: $workspaceStore ?? '',
							id: jobId,
							requestBody: {}
						}))
				} catch {}
				jobId = undefined
			}}
			size="md"
		>
			Cancel
		</Button>
	{:else}
		<Button
			variant="contained"
			endIcon={{ icon: intervalState === 'done' ? faRefresh : faPlay }}
			size="lg"
			color="blue"
			btnClasses="w-full"
			disabled={!isValid}
			on:click={() => runPreview($previewArgs)}
		>
			{`Run${intervalState === 'done' ? ' again' : ''}`}
		</Button>
	{/if}

	<div class="h-full overflow-y-auto mb-16 grow">
		{#if jobId}
			<FlowStatusViewer
				{jobId}
				on:jobsLoaded={(e) => {
					intervalState = 'done'
					const parentIndex = selectedIdToIndexes($selectedId)[0]
					const upToIndex =
						previewMode === 'upTo' ? Number(parentIndex) + 1 : $flowStateStore.modules.length
					mapJobResultsToFlowState(e.detail, upToIndex)
				}}
			/>
		{/if}
	</div>
</div>
