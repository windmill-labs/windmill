<script lang="ts">
	import { faHourglassHalf, faSpinner, faTimes } from '@fortawesome/free-solid-svg-icons'
	import { truncateRev } from '$lib/utils'

	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'

	import { CompletedJob, FlowModuleValue, FlowStatusModule, JobService, QueuedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import DisplayResult from './DisplayResult.svelte'
	import ChevronButton from './ChevronButton.svelte'
	import JobStatus from './JobStatus.svelte'

	export let job: QueuedJob | CompletedJob
	export let jobs: (CompletedJob | undefined)[]

	function loadResults() {
		job?.flow_status?.modules?.forEach(async (x, i) => {
			if (
				(i >= jobs.length && x.type == FlowStatusModule.type.SUCCESS) ||
				x.type == FlowStatusModule.type.FAILURE
			) {
				jobs.push(undefined)
				jobs[i] = await JobService.getCompletedJob({ workspace: $workspaceStore!, id: x.job! })
			}
		})
	}

	$: $workspaceStore && job && loadResults()
</script>

<div class="flow-root max-w-lg w-full p-4">
	<div class="flex flex-row-reverse">
		{#if job}
			<div class="flex-col">
				<a href="/run/{job?.id}" class="font-medium text-blue-600"
					>{truncateRev(job?.id ?? '', 10)}</a
				>
				<div>
					<JobStatus {job} />
				</div>
			</div>
		{/if}
	</div>

	<p class="text-gray-500 mb-6 w-full text-center">
		Step
		<span class="font-medium text-gray-900"
			>{Math.min((job?.flow_status?.step ?? 0) + 1, job?.raw_flow?.modules.length ?? 0)}</span
		>
		out of <span class="font-medium text-gray-900">{job?.raw_flow?.modules.length}</span>
		<span class="mt-4" />
	</p>
	<ul role="list" class="-mb-8 w-full">
		{#each job?.raw_flow?.modules ?? [] as mod, i}
			<li class="w-full">
				<div class="relative pb-8 w-full">
					{#if i < (job?.raw_flow?.modules ?? []).length - 1}
						<span
							class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200"
							aria-hidden="true"
						/>
					{/if}
					<div class="relative flex space-x-3">
						<div>
							{#if job.flow_status?.modules[i].type == FlowStatusModule.type.SUCCESS}
								<span
									class="h-8 w-8 rounded-full bg-green-600 flex items-center justify-center ring-8 ring-white"
								>
									<Icon
										class="text-white"
										data={check}
										scale={0.8}
										label="Job completed successfully"
									/>
								</span>
							{:else if job.flow_status?.modules[i].type == FlowStatusModule.type.FAILURE}
								<span
									class="h-8 w-8 rounded-full bg-red-600 flex items-center justify-center ring-8 ring-white"
								>
									<Icon class="text-white" data={faTimes} scale={0.8} label="Job failed" />
								</span>
							{:else if job.flow_status?.modules[i].type == FlowStatusModule.type.IN_PROGRESS}
								<span
									class="h-8 w-8 rounded-full bg-yellow-500 flex items-center justify-center ring-8 ring-white"
								>
									<Icon
										class="text-white animate-spin"
										data={faSpinner}
										scale={1}
										label="Job failed"
									/>
								</span>
							{:else}
								<span
									class="h-8 w-8 rounded-full bg-gray-400 flex items-center justify-center ring-8 ring-white"
								>
									<Icon class="text-white" data={faHourglassHalf} scale={1} label="Job failed" />
								</span>
							{/if}
						</div>
						<div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4 w-full">
							<div class="w-full">
								<p class="text-sm text-gray-500">
									{#if mod.value.type == FlowModuleValue.type.SCRIPT}
										Script at path <a
											href="/scripts/get/{mod.value.path}"
											class="font-medium text-gray-900">{mod.value.path}</a
										>
									{/if}
								</p>
							</div>
							<div class="text-right text-sm whitespace-nowrap text-gray-500">
								{job.flow_status?.modules[i].type}
								{#if job.flow_status?.modules[i].job}
									<a href="/run/{job.flow_status?.modules[i].job}" class="font-medium text-blue-600"
										>{truncateRev(job.flow_status?.modules[i].job ?? '', 10)}</a
									>
								{/if}
							</div>
						</div>
					</div>
					{#if i < jobs.length && jobs[i] != undefined}
						<div class="flex flex-col ml-10">
							<div>
								<ChevronButton text="result" viewOptions={true}>
									<div class="text-xs">
										<DisplayResult result={jobs[i]?.result} />
									</div>
								</ChevronButton>
							</div>
							<div>
								<ChevronButton text="logs">
									<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-lg">
										<pre class="w-full">{jobs[i]?.logs}</pre>
									</div>
								</ChevronButton>
							</div>
						</div>
					{/if}
				</div>
			</li>
		{/each}
	</ul>
</div>
