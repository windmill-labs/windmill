<script lang="ts">
	import { FlowStatusModule, Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import Icon from 'svelte-awesome'
	import { faChevronDown, faChevronUp, faHourglassHalf } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import { onDestroy } from 'svelte'
	import type { FlowState } from './flows/flowState'
	import { Button } from './common'

	const dispatch = createEventDispatcher()

	export let jobId: string

	export let flowState: FlowState | undefined = undefined
	export let flowJobIds:
		| {
				moduleId: string
				flowJobs: string[]
		  }
		| undefined = undefined
	export let job: Job | undefined = undefined

	let forloop_selected = ''
	let timeout: NodeJS.Timeout

	$: innerModules =
		job?.flow_status?.modules
			.filter((x) => x.job != jobId)
			.concat(
				job?.flow_status.failure_module.type != 'WaitingForPriorSteps'
					? job?.flow_status.failure_module
					: []
			) ?? []

	async function loadJobInProgress() {
		if (jobId != '00000000-0000-0000-0000-000000000000') {
			job = await JobService.getJob({
				workspace: $workspaceStore ?? '',
				id: jobId ?? ''
			})
		}
		if (job?.type !== 'CompletedJob') {
			timeout = setTimeout(() => loadJobInProgress(), 500)
		}
	}

	$: job && dispatch('jobsLoaded', job)

	function updateJobId() {
		if (jobId !== job?.id) {
			loadJobInProgress()
		}
	}

	$: jobId && updateJobId()

	onDestroy(() => {
		timeout && clearTimeout(timeout)
	})
</script>

{#if job}
	<div class="flow-root w-full space-y-4 ">
		{#if innerModules.length > 0}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b pb-2">Flow result</h3>
		{/if}
		<FlowPreviewStatus {job} />
		{#if `result` in job}
			<div class="w-full h-full overflow-auto max-h-80 bg-white">
				<FlowJobResult {job} />
			</div>
		{:else if job.logs}
			<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
				<pre class="w-full">{job.logs}</pre>
			</div>
		{/if}

		{#if flowJobIds && Array.isArray(flowJobIds?.flowJobs) && flowJobIds?.flowJobs.length > 0}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4">
				Embedded flows: ({flowJobIds?.flowJobs.length} items)
			</h3>
			{#each flowJobIds.flowJobs as loopJobId, j}
				<Button
					variant={forloop_selected === loopJobId ? 'contained' : 'border'}
					color={forloop_selected === loopJobId ? 'dark' : 'light'}
					btnClasses="w-full flex justify-start"
					on:click={() => {
						if (forloop_selected == loopJobId) {
							forloop_selected = ''
						} else {
							forloop_selected = loopJobId
						}
					}}
				>
					<span class="truncate">
						#{j + 1}: {loopJobId}
					</span>

					<Icon
						class="ml-2"
						data={forloop_selected == loopJobId ? faChevronUp : faChevronDown}
						scale={0.8}
					/>
				</Button>
				<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
					<svelte:self
						{flowState}
						jobId={loopJobId}
						on:jobsLoaded={(e) => {
							if (flowJobIds?.moduleId) {
								if (flowState) {
									if (
										!flowState[flowJobIds.moduleId].previewResult ||
										!Array.isArray(flowState[flowJobIds.moduleId]?.previewResult)
									) {
										flowState[flowJobIds.moduleId].previewResult = []
									}
									flowState[flowJobIds.moduleId].previewResult[j] = e.detail.result
									flowState[flowJobIds.moduleId].previewArgs = e.detail.args
								}
							}
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
					<div class="line w-8 h-10" />
					<h3 class="text-gray-500 mb-2 w-full">
						{#if job?.raw_flow?.modules && i < job?.raw_flow?.modules.length}
							Step
							<span class="font-medium text-gray-900">
								{i + 1}
							</span>
							out of
							<span class="font-medium text-gray-900">{job?.raw_flow?.modules.length}</span>
							{#if job.raw_flow?.modules[i]?.summary}
								: <span class="font-medium text-gray-900">
									{job.raw_flow?.modules[i]?.summary ?? ''}
								</span>
							{/if}
						{:else}
							<h3>Failure module</h3>
						{/if}
					</h3>
					<div class="line w-8 h-10" />
					<li class="w-full border p-6 space-y-2 bg-blue-50/50">
						{#if [FlowStatusModule.type.IN_PROGRESS, FlowStatusModule.type.SUCCESS, FlowStatusModule.type.FAILURE].includes(mod.type)}
							<svelte:self
								{flowState}
								jobId={mod.job}
								flowJobIds={mod.flow_jobs
									? {
											moduleId: mod.id,
											flowJobs: mod.flow_jobs
									  }
									: undefined}
								on:jobsLoaded={(e) => {
									if (mod.id && (mod.flow_jobs ?? []).length == 0) {
										if (flowState && flowState[mod.id]) {
											flowState[mod.id].previewResult = e.detail.result
											flowState[mod.id].previewArgs = e.detail.args
										}
									}
								}}
							/>
						{:else}
							<span class="italic text-gray-600">
								<Icon data={faHourglassHalf} class="mr-2" />

								{#if mod.type == FlowStatusModule.type.WAITING_FOR_EVENT}
									Waiting to be resumed by receivent events such as approvals
								{:else if mod.type == FlowStatusModule.type.WAITING_FOR_PRIOR_STEPS}
									Waiting for prior steps to complete
								{:else if mod.type == FlowStatusModule.type.WAITING_FOR_EXECUTOR}
									Job is ready to be executed and will be picked up by the next available worker
								{/if}
							</span>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
{:else}
	Job loading...
{/if}

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
