<script lang="ts">
	import { scriptPathToHref, truncateRev } from '$lib/utils'
	import { faHourglassHalf, faSpinner, faTimes } from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'

	import { CompletedJob, FlowStatusModule, Job, JobService, QueuedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import IconedPath from './IconedPath.svelte'
	import { createEventDispatcher } from 'svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	const dispatch = createEventDispatcher()

	export let job: QueuedJob | CompletedJob
	export let fullyRetrieved = -1

	let lastJobid: string | undefined
	let forloop_selected = ''
	let pres: { [key: number]: HTMLElement } = {}

	$: jobs = [] as Array<any>
	$: jobs && dispatch('jobsLoaded', jobs)
	$: $workspaceStore && job && loadResults()

	async function loadResults() {
		if (!('success' in job)) {
			const modules = job?.flow_status?.modules
			if (modules) {
				let stepInProgress = modules?.findIndex(
					(module) => module.type == FlowStatusModule.type.IN_PROGRESS
				)
				if (stepInProgress != -1) {
					const last = modules[stepInProgress]
					jobs[stepInProgress] = await JobService.getJob({
						workspace: $workspaceStore ?? '',
						id: last.job ?? ''
					})
					jobs = jobs
					pres[stepInProgress]?.scroll({
						top: pres[stepInProgress]?.scrollHeight,
						behavior: 'smooth'
					})
				}
			}
		}
		if (job.id != lastJobid) {
			lastJobid = job.id
			jobs = []
			fullyRetrieved = -1
		}

		job?.flow_status?.modules?.forEach(async (module, index) => {
			const previewDone =
				(index > fullyRetrieved && module.type == FlowStatusModule.type.SUCCESS) ||
				module.type == FlowStatusModule.type.FAILURE

			if (previewDone) {
				const completedJob = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: module.job!
				})
				if (module.forloop_jobs) {
					const forloop_jobs: CompletedJob[] = []

					for (let j of module.forloop_jobs) {
						forloop_jobs.push(
							await JobService.getCompletedJob({ workspace: $workspaceStore!, id: j })
						)
					}
					jobs[index] = forloop_jobs
				} else {
					jobs[index] = completedJob
				}
				jobs = jobs
				fullyRetrieved = index
			}
		})
	}

	const hasModules = Array.isArray(job?.raw_flow?.modules) && job?.raw_flow?.modules.length! > 1
</script>

<div class="flow-root w-full space-y-4">
	<h3 class="text-md leading-6 font-bold text-gray-900 border-b pb-2">Preview results</h3>

	<FlowPreviewStatus {job} />

	{#if `result` in job}
		<FlowJobResult {job} />
	{/if}

	<ul class="w-full">
		{#if hasModules}
			<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4 py-2">Detailed results</h3>

			{#each job?.raw_flow?.modules ?? [] as mod, i}
				<p class="text-gray-500 mb-6 w-full ">
					Step
					<span class="font-medium text-gray-900"> {i + 1} </span> out of
					<span class="font-medium text-gray-900">{job?.raw_flow?.modules.length}</span>
					<span class="mt-4" />
				</p>

				<li class="w-full border ">
					<div>
						{#if mod.value.type == 'script'}
							<div class="flex px-6 pt-4 align-middle items-center">
								<span>Script at path</span><a
									target="_blank"
									href={scriptPathToHref(mod.value.path ?? '')}
									class="font-medium text-gray-900 flex"
								>
									<IconedPath path={mod.value.path} />
								</a>
							</div>
						{/if}

						{#if jobs[i]}
							{#if Array.isArray(jobs[i])}
								<div class="flex flex-col space-y-2 max-h-60 overflow-y-auto">
									{#each jobs[i] as job, i}
										<button
											class="underline text-blue-600 hover:text-blue-700"
											class:text-red-600={!job.success}
											on:click={() => {
												if (forloop_selected == job.id) {
													forloop_selected = ''
												} else {
													forloop_selected = job.id
												}
											}}
										>
											Iteration: #{i}: {job.id}
											{forloop_selected == job.id ? '(-)' : '(+)'}
										</button>
										{#if forloop_selected == job.id}
											<svelte:self {job} />
										{/if}
									{/each}
								</div>
							{:else if jobs[i].type == 'CompletedJob'}
								<div class="m-6">
									<svelte:self job={jobs[i]} />
								</div>
							{:else if jobs[i]}
								{#if jobs[i]?.raw_flow}
									<div class="border-2">
										<h2>Forloop current iteration</h2>
										<svelte:self job={jobs[i]} />
									</div>
								{:else}
									<div class="p-6">
										<svelte:self job={jobs[i]} />
									</div>
								{/if}
							{/if}
						{/if}
					</div>
				</li>
			{/each}
		{/if}
	</ul>
</div>
