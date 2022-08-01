<script lang="ts">
	import type { Schema } from '$lib/common'
	import { InputTransform, Job, JobService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import FlowJobResult from './FlowJobResult.svelte'
	import type { FlowMode } from './flows/flowStore'
	import { flowToMode } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import RunForm from './RunForm.svelte'
	import Tabs from './Tabs.svelte'

	const dispatch = createEventDispatcher()
	export let i: number
	export let flow: Flow
	export let schemas: Schema[] = []
	export let mode: FlowMode

	export let args: Record<string, any> = {}

	let stepArgs: Record<string, any> = {}

	let tab: 'upto' | 'justthis' = 'upto'
	let viewPreview = false
	let intervalId: NodeJS.Timer

	let uptoText =
		i >= flow.value.modules.length - 1 ? 'Preview whole flow' : 'Preview up to this step'
	let job: Job | undefined
	let jobs = []
	let jobId: string

	$: dispatch('change', jobs)

	export async function runPreview(args) {
		viewPreview = true
		intervalId && clearInterval(intervalId)
		let newFlow = flowToMode(flow, mode)
		newFlow = tab == 'upto' ? truncateFlow(newFlow) : extractStep(newFlow)
		jobId = await JobService.runFlowPreview({
			workspace: $workspaceStore ?? '',
			requestBody: {
				args,
				value: newFlow.value,
				path: newFlow.path
			}
		})
		jobs = []
		intervalId = setInterval(loadJob, 1000)
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
		localFlow.schema = schemas[i]
		stepArgs = {}
		Object.entries(flow.value.modules[i].input_transform).forEach((x) => {
			if (x[1].type == InputTransform.type.STATIC) {
				stepArgs[x[0]] = x[1].value
			}
		})
		return localFlow
	}

	async function loadJob() {
		try {
			job = await JobService.getJob({ workspace: $workspaceStore!, id: jobId })
			if (job?.type == 'CompletedJob') {
				//only CompletedJob has success property
				clearInterval(intervalId)
			}
		} catch (err) {
			sendUserToast(err, true)
		}
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})
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
		<Tabs
			tabs={[
				['upto', uptoText],
				['justthis', 'preview just this step']
			]}
			bind:tab
		/>
	{/if}
	<div class="my-2" />
	{#if tab == 'upto'}
		<RunForm
			runnable={truncateFlow(flow)}
			runAction={(_, args) => runPreview(args)}
			schedulable={false}
			buttonText={uptoText}
			detailed={false}
			bind:args
		/>
	{:else}
		<RunForm
			runnable={extractStep(flow)}
			runAction={(_, args) => runPreview(args)}
			schedulable={false}
			buttonText="Preview just this step"
			detailed={false}
			args={stepArgs}
		/>
	{/if}

	{#if job}
		<div class="w-full flex justify-center">
			<FlowStatusViewer {job} bind:jobs />
		</div>
		{#if `result` in job}
			<FlowJobResult {job} />
		{/if}
	{/if}
{/if}
