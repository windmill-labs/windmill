<script lang="ts">
	import { JobService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faClose, faPlay, faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { Button } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { flowStateStore } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import { mapJobResultsToFlowState, type JobResult } from './flows/previousResults'

	export let previewMode: 'upTo' | 'whole'

	let jobId: string | undefined = undefined
	let isValid: boolean = false
	let isRunning: boolean = false

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	function extractFlow(previewMode: 'upTo' | 'whole'): Flow {
		/*
		if (previewMode === 'whole') {
			return $flowStore
		} else {
			const positions = $idToPositionStore.get($selectedId)!

			if (positions.length < 2) {
				throw new Error('Up to inside branch is not supported, nor deeply nested modules')
			}

			const [parentIndex, childIndex] = positions

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
		*/

		const flow = JSON.parse(JSON.stringify($flowStore))

		return flow
	}

	const dispatch = createEventDispatcher()

	export async function runPreview(args: Record<string, any>) {
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
		if (jobResult.job?.type === 'CompletedJob') {
			isRunning = false
		}

		// TODO
		const upToIndex = 1

		mapJobResultsToFlowState(jobResult, upToIndex)
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
			size="lg"
			color="blue"
			btnClasses="w-full"
			disabled={!isValid}
			on:click={() => runPreview($previewArgs)}
		>
			Test flow (Ctrl/Cmd + Enter)
		</Button>
	{/if}

	<div class="h-full overflow-y-auto mb-16 grow">
		{#if jobId}
			<FlowStatusViewer {jobId} on:jobsLoaded={(e) => onJobsLoaded(e.detail)} />
		{/if}
	</div>
</div>
