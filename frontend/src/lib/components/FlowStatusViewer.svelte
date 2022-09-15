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
	export let root: boolean = false
	export let forloopJobIds: string[] | undefined = undefined
	export let jobResult: JobResult = {
		job: undefined,
		innerJobs: [],
		loopJobs: []
	}

	let forloop_selected = ''
	let timeout: NodeJS.Timeout

	async function loadJobInProgress() {
		const job = await JobService.getJob({
			workspace: $workspaceStore ?? '',
			id: jobId ?? ''
		})

		jobResult.job = job
		jobResult = jobResult

		if (job?.type !== 'CompletedJob') {
			timeout = setTimeout(() => loadJobInProgress(), 500)
		} else if (root) {
			dispatch('jobsLoaded', jobResult)
		}
	}

	$: hasModules =
		jobResult.job &&
		Array.isArray(jobResult.job?.raw_flow?.modules) &&
		jobResult.job?.raw_flow?.modules.length! > 1

	function updateJobId() {
		if (jobId !== jobResult.job?.id) {
			loadJobInProgress()
		}
	}

	$: jobId && updateJobId()

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})
</script>

{#if jobResult.job}
	<div class="flow-root w-full space-y-4">
		<h3 class="text-md leading-6 font-bold text-gray-900 border-b pb-2">Preview results</h3>
		<FlowPreviewStatus job={jobResult.job} />
		{#if `result` in jobResult.job}
			<FlowJobResult job={jobResult.job} />
		{:else if jobResult.job.logs}
			<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
				<pre class="w-full">{jobResult.job.logs}</pre>
			</div>
		{/if}

		{#if Array.isArray(forloopJobIds) && forloopJobIds?.length > 0 && Array.isArray(jobResult.loopJobs)}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4">
				Loop results ({forloopJobIds.length} items)
			</h3>
			{#each forloopJobIds as loopJobId, j}
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
					<svelte:self jobId={loopJobId} bind:jobResult={jobResult.loopJobs[j]} />
				</div>
			{/each}
		{:else if hasModules && Array.isArray(jobResult.innerJobs)}
			<ul class="w-full">
				<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4 py-2">
					Detailed results
				</h3>

				{#each jobResult.job?.flow_status?.modules ?? [] as mod, i}
					<p class="text-gray-500 mb-6 w-full ">
						Step
						<span class="font-medium text-gray-900"> {i + 1} </span> out of
						<span class="font-medium text-gray-900">{jobResult.job?.raw_flow?.modules.length}</span>
					</p>

					<li class="w-full border p-6 space-y-2">
						{#if [FlowStatusModule.type.IN_PROGRESS, FlowStatusModule.type.SUCCESS, FlowStatusModule.type.FAILURE].includes(mod.type)}
							<svelte:self
								jobId={mod.job}
								bind:jobResult={jobResult.innerJobs[i]}
								forloopJobIds={mod.forloop_jobs}
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
	No script selected
{/if}
