<script lang="ts">
	import { FlowStatusModule, Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import Icon from 'svelte-awesome'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import { onDestroy } from 'svelte'
	import { flowStateStore } from './flows/flowState'
	import Button from './common/button/Button.svelte'

	const dispatch = createEventDispatcher()

	export let jobId: string

	export let flowJobIds:
		| {
				moduleId: string
				flowJobs: string[]
		  }
		| undefined = undefined
	export let job: Job | undefined = undefined

	let forloop_selected = ''
	let timeout: NodeJS.Timeout

	$: innerModules = job?.flow_status?.modules ?? []

	async function loadJobInProgress() {
		job = await JobService.getJob({
			workspace: $workspaceStore ?? '',
			id: jobId ?? ''
		})

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
	<div class="flow-root w-full space-y-4">
		{#if innerModules.length > 0}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b pb-2">Flow result</h3>
		{/if}
		<FlowPreviewStatus {job} />
		{#if `result` in job}
			<FlowJobResult {job} />
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
					#{j}: {loopJobId}

					<Icon
						class="ml-2"
						data={forloop_selected == loopJobId ? faChevronUp : faChevronDown}
						scale={0.8}
					/>
				</Button>
				<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
					<svelte:self
						jobId={loopJobId}
						on:jobsLoaded={(e) => {
							if (flowJobIds?.moduleId) {
								$flowStateStore[flowJobIds.moduleId].previewResult = e.detail.result
								$flowStateStore[flowJobIds.moduleId].previewArgs = e.detail.args
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
					<p class="text-gray-500 mb-2 w-full ">
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
							<span class="font-medium text-gray-900"> Failure module </span>
						{/if}
					</p>

					<li class="w-full border p-6 space-y-2">
						{#if [FlowStatusModule.type.IN_PROGRESS, FlowStatusModule.type.SUCCESS, FlowStatusModule.type.FAILURE].includes(mod.type)}
							<svelte:self
								jobId={mod.job}
								flowJobIds={mod.flow_jobs
									? {
											moduleId: mod.id,
											flowJobs: mod.flow_jobs
									  }
									: undefined}
								on:jobsLoaded={(e) => {
									if (mod.id) {
										$flowStateStore[mod.id].previewResult = e.detail.result
										$flowStateStore[mod.id].previewArgs = e.detail.args
									}
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
