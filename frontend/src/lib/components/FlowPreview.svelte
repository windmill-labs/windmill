<script lang="ts">
	import type { Schema } from '$lib/common'
	import { InputTransform, Job, JobService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import ChevronButton from './ChevronButton.svelte'
	import DisplayResult from './DisplayResult.svelte'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import RunForm from './RunForm.svelte'
	import Tabs from './Tabs.svelte'

	const dispatch = createEventDispatcher()
	export let i: number
	export let flow: Flow
	export let schemas: Schema[] = []

	export let args: Record<string, any> = {}

	let stepArgs: Record<string, any> = {}

	let tab: 'upto' | 'justthis' = 'upto'
	let viewPreview = false
	let intervalId: NodeJS.Timer

	let uptoText =
		i == flow.value.modules.length - 1 ? 'Preview whole flow' : 'Preview up to this step'
	let job: Job | undefined
	let jobs = []
	let jobId: string

	async function runPreview(args) {
		intervalId && clearInterval(intervalId)
		const newFlow = tab == 'upto' ? truncateFlow(flow) : extractStep(flow)
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
			dispatch('change', job)
		} catch (err) {
			sendUserToast(err, true)
		}
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})
</script>

<h2 class="mb-5 mt-2">
	<button
		type="submit"
		class="underline text-gray-700 inline-flex  items-center"
		on:click={() => {
			viewPreview = !viewPreview
		}}
	>
		<div>
			Preview mode<Icon class="ml-1" data={viewPreview ? faChevronUp : faChevronDown} scale={1} />
		</div>
	</button>
</h2>

{#if viewPreview}
	{#if i != flow.value.modules.length - 1}
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
			<div class="flex flex-col ml-10">
				<div>
					<ChevronButton text="result" viewOptions={true}>
						<div class="text-xs">
							<DisplayResult result={job.result} />
						</div>
					</ChevronButton>
				</div>
				<div>
					<ChevronButton text="logs" viewOptions={true}>
						<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-lg">
							<pre class="w-full">{job.logs}</pre>
						</div>
					</ChevronButton>
				</div>
			</div>
		{/if}
	{/if}
{/if}
