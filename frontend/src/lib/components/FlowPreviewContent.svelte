<script lang="ts">
	import { FlowStatusModule, JobService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faClose, faPlay, faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { Button } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { flowStateStore } from './flows/flowState'
	import { mapJobResultsToFlowState, type JobResult } from './flows/flowStateUtils'
	import { flowStore } from './flows/flowStore'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview, selectedIdToIndexes } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import { ProgressBar, type GeneralStep, type LoopStep, type ProgressStep } from './progressBar'

	export let previewMode: 'upTo' | 'whole'

	let jobId: string | undefined = undefined
	let isValid: boolean = false
	let isRunning: boolean = false
	let jobProgress: { steps: ProgressStep[]; reset?: () => void } = { steps: [] }

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	function extractFlow(previewMode: 'upTo' | 'whole'): Flow {
		if (previewMode === 'whole') {
			return $flowStore
		} else {
			let [parentIndex, childIndex] = selectedIdToIndexes($selectedId)

			const flow = JSON.parse(JSON.stringify($flowStore))
			const modules = flow.value.modules.slice(0, Number(parentIndex) + 1)
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
		resetJobProgress()
		const newFlow = extractFlow(previewMode)
		jobId = await runFlowPreview(args, newFlow)
		isRunning = true
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey) {
					event.preventDefault()
					runPreview($previewArgs)
				}
				break

			case 'Escape':
				dispatch('close')
				break
		}
	}

	function onJobsLoaded(jobResult: JobResult) {
		updateJobProgress(jobResult)

		if (jobResult.job?.type === 'CompletedJob') {
			isRunning = false
		}
		const upToIndex =
			previewMode === 'upTo'
				? selectedIdToIndexes($selectedId)[0] + 1
				: $flowStateStore.modules.length
		mapJobResultsToFlowState(jobResult, upToIndex)
	}

	function updateJobProgress(job: JobResult) {
		const modules = job.job?.flow_status?.modules
		if (!modules?.length) {
			return
		}

		jobProgress.steps = modules.map(({ type, iterator }) => {
			if (iterator) {
				return <LoopStep>{
					type: 'loop',
					isDone: isJobStepDone(type),
					index: iterator.index || 0,
					length: iterator.itered?.length || 0
				}
			}
			return <GeneralStep>{
				type: 'general',
				isDone: isJobStepDone(type)
			}
		})
	}

	function isJobStepDone(type: FlowStatusModule.type | undefined) {
		if (!type) {
			return false
		}
		return type === FlowStatusModule.type.SUCCESS || type === FlowStatusModule.type.FAILURE
	}

	function resetJobProgress() {
		jobProgress.steps = []
		jobProgress?.reset && jobProgress.reset()
	}
</script>

<svelte:window on:keydown={onKeyDown} />

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
	{#if isRunning}
		<Button
			disabled={!isValid}
			color="red"
			on:click={async () => {
				isRunning = false
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
			endIcon={{ icon: isRunning ? faRefresh : faPlay }}
			color="blue"
			btnClasses="w-full"
			disabled={!isValid}
			on:click={() => runPreview($previewArgs)}
		>
			Test flow (Ctrl/Cmd + Enter)
		</Button>
	{/if}

	<ProgressBar steps={jobProgress.steps} bind:reset={jobProgress.reset} class="py-4" />
	<!-- {#if jobProgress?.steps?.length}
	{/if} -->

	<div class="h-full overflow-y-auto mb-16 grow">
		{#if jobId}
			<FlowStatusViewer {jobId} on:jobsLoaded={({ detail }) => onJobsLoaded(detail)} />
		{/if}
	</div>
</div>
