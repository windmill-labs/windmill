<script lang="ts">
	import { FlowStatusModule, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import { Button } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import type { JobResult } from './flows/flowStateUtils'
	import { onDestroy } from 'svelte'

	const dispatch = createEventDispatcher()

	export let jobId: string
	export let flowJobIds: string[] | undefined = undefined
	export let jobResult: JobResult = {
		job: undefined,
		innerJobs: [],
		loopJobs: []
	}

	let forloop_selected = ''
	let timeout: NodeJS.Timeout

	$: innerModules = jobResult?.job?.flow_status?.modules ?? []

	async function loadJobInProgress() {
		const job = await JobService.getJob({
			workspace: $workspaceStore ?? '',
			id: jobId ?? ''
		})

		if (jobResult) {
			jobResult.job = job
			jobResult = jobResult
		} else {
			jobResult = {
				job: job,
				innerJobs: [],
				loopJobs: []
			}
		}

		if (job?.type !== 'CompletedJob') {
			timeout = setTimeout(() => loadJobInProgress(), 500)
		}
	}

	$: jobResult && dispatch('jobsLoaded', jobResult)

	function updateJobId() {
		if (jobId !== jobResult?.job?.id) {
			loadJobInProgress()
		}
	}

	$: jobId && updateJobId()

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})
</script>

{#if jobResult?.job}
	<div class="flow-root w-full space-y-4">
		{#if innerModules.length > 0}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b pb-2">Flow result</h3>
		{/if}
		<FlowPreviewStatus job={jobResult.job} />
		{#if `result` in jobResult.job}
			<FlowJobResult job={jobResult.job} />
		{:else if jobResult.job.logs}
			<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
				<pre class="w-full">{jobResult.job.logs}</pre>
			</div>
		{/if}

		{#if Array.isArray(flowJobIds) && flowJobIds?.length > 0 && Array.isArray(jobResult.loopJobs)}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4">
				Loop results ({flowJobIds.length} items)
			</h3>
			{#each flowJobIds as loopJobId, j}
				<Button
					color={forloop_selected == loopJobId ? 'dark' : 'light'}
					class="flex justify-between w-full"
					on:click={() => {
						if (forloop_selected == loopJobId) {
							forloop_selected = ''
						} else {
							forloop_selected = loopJobId
						}
					}}
				>
					Iteration: #{j}: {loopJobId}

					<Icon
						class="ml-2"
						data={forloop_selected == loopJobId ? faChevronUp : faChevronDown}
						scale={0.8}
					/>
				</Button>
				<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
					<svelte:self
						jobId={loopJobId}
						bind:jobResult={jobResult.loopJobs[j]}
						on:jobsLoaded={(e) => {
							jobResult = jobResult
						}}
					/>
				</div>
			{/each}
		{:else if innerModules.length > 0}
			<ul class="w-full">
				<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4 py-2">
					Step-by-step results
				</h3>

				{#each innerModules as mod, i}
					<p class="text-gray-500 mb-2 w-full ">
						{#if jobResult.job?.raw_flow?.modules && i < jobResult.job?.raw_flow?.modules.length}
							Step
							<span class="font-medium text-gray-900">
								{i + 1}
							</span>
							out of
							<span class="font-medium text-gray-900"
								>{jobResult.job?.raw_flow?.modules.length}</span
							>
							{#if jobResult.job.raw_flow?.modules[i]?.summary}
								: <span class="font-medium text-gray-900"
									>{jobResult.job.raw_flow?.modules[i]?.summary ?? ''}</span
								>
							{/if}
						{:else}
							<span class="font-medium text-gray-900"> Failure module </span>
						{/if}
					</p>

					<li class="w-full border p-6 space-y-2">
						{#if [FlowStatusModule.type.IN_PROGRESS, FlowStatusModule.type.SUCCESS, FlowStatusModule.type.FAILURE].includes(mod.type)}
							<svelte:self
								jobId={mod.job}
								bind:jobResult={jobResult.innerJobs[i]}
								flowJobIds={mod.flow_jobs}
								on:jobsLoaded={(e) => {
									jobResult = jobResult
								}}
							/>
						{:else}
							<span>{mod.type}</span>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{:else}
	Job loading...
{/if}
