<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	import { faClose, faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher, getContext, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import { flowStateStore, flowStateToFlow, type FlowModuleSchema } from './flows/flowState'
	import { mapJobResultsToFlowState } from './flows/flowStateUtils'
	import { flowStore } from './flows/flowStore'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import SchemaForm from './SchemaForm.svelte'

	export let previewMode: 'upTo' | 'whole'

	let args: Record<string, any> = {}

	let jobId: string | undefined = undefined
	let isValid: boolean = false
	let intervalState: 'idle' | 'canceled' | 'done' | 'running' = 'idle'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	function extractFlow(previewMode: 'upTo' | 'whole') {
		if (previewMode === 'whole') {
			return flowStateToFlow($flowStateStore, $flowStore)
		} else {
			const [parentIndex, childIndex] = $selectedId.split('-')

			if (childIndex === undefined) {
				const modules = $flowStateStore.modules.slice(0, Number(parentIndex) + 1)
				const flowState = {
					modules: modules,
					failureModule: $flowStateStore.failureModule
				}
				return flowStateToFlow(flowState, $flowStore)
			} else {
				const modules = $flowStateStore.modules.slice(0, Number(parentIndex) + 1)
				const flowModuleSchemas: FlowModuleSchema[] = JSON.parse(JSON.stringify(modules))

				const flowModuleSchema = flowModuleSchemas[modules.length - 1]

				flowModuleSchemas[modules.length - 1] = {
					...flowModuleSchemas[modules.length - 1],
					childFlowModules: flowModuleSchema.childFlowModules!.slice(0, Number(childIndex) + 1)
				}

				if (flowModuleSchema.flowModule.value.type === 'forloopflow') {
					flowModuleSchema.flowModule.value.modules =
						flowModuleSchema.flowModule.value.modules.slice(0, Number(childIndex) + 1)

					flowModuleSchemas[modules.length - 1].flowModule = flowModuleSchema.flowModule
				}

				const flowState = {
					modules: flowModuleSchemas,
					failureModule: $flowStateStore.failureModule
				}
				return flowStateToFlow(flowState, $flowStore)
			}
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
			color="alternative"
			on:click={() => {
				jobId = undefined
				intervalState = 'idle'
				dispatch('close')
			}}
		>
			<Icon data={faClose} />
		</Button>
	</div>
	<div class="pb-4">
		<div class="mt-4">
			<SchemaForm schema={$flowStore.schema} bind:isValid bind:args />
		</div>
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
		<Button disabled={!isValid} class="blue-button" on:click={() => runPreview(args)} size="md">
			{`Run${intervalState === 'done' ? ' again' : ''}`}
		</Button>
	{/if}

	<div class="h-full overflow-y-auto mb-16 grow">
		{#if jobId}
			<FlowStatusViewer
				{jobId}
				on:jobsLoaded={(e) => {
					intervalState = 'done'
					const [parentIndex] = $selectedId.split('-')
					const configIndex =
						previewMode === 'upTo' ? Number(parentIndex) : $flowStateStore.modules.length
					mapJobResultsToFlowState(e.detail, 'upto', configIndex, undefined)
				}}
				root={true}
			/>
		{/if}
	</div>
</div>
