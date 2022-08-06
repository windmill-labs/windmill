<script lang="ts">
	import { scriptPathToHref, truncateRev } from '$lib/utils'
	import { faHourglassHalf, faSpinner, faTimes } from '@fortawesome/free-solid-svg-icons'

	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'

	import { CompletedJob, FlowStatusModule, Job, JobService, QueuedJob } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import JobStatus from './JobStatus.svelte'

	export let job: QueuedJob | CompletedJob
	export let jobs: (Job | Job[] | undefined)[] = []
	export let fullyRetrieved = -1

	let lastJobid: string | undefined

	let forloop_selected = ''

	let pres: { [key: number]: HTMLElement } = {}

	async function loadResults() {
		if (!('success' in job)) {
			const mods = job?.flow_status?.modules
			if (mods) {
				let i = mods?.findIndex((x) => x.type == FlowStatusModule.type.IN_PROGRESS)
				if (i != -1) {
					let last = mods[i]
					jobs[i] = await JobService.getJob({
						workspace: $workspaceStore ?? '',
						id: last.job ?? ''
					})
					jobs = jobs
					pres[i]?.scroll({ top: pres[i]?.scrollHeight, behavior: 'smooth' })
				}
			}
		}
		if (job.id != lastJobid) {
			lastJobid = job.id
			jobs = []
			fullyRetrieved = -1
		}
		job?.flow_status?.modules?.forEach(async (x, i) => {
			if (
				(i > fullyRetrieved && x.type == FlowStatusModule.type.SUCCESS) ||
				x.type == FlowStatusModule.type.FAILURE
			) {
				if (x.forloop_jobs) {
					const forloop_jobs: CompletedJob[] = []

					for (let j of x.forloop_jobs) {
						forloop_jobs.push(
							await JobService.getCompletedJob({ workspace: $workspaceStore!, id: j })
						)
					}
					jobs[i] = forloop_jobs
				} else {
					jobs[i] = await JobService.getCompletedJob({ workspace: $workspaceStore!, id: x.job! })
				}
				jobs = jobs
				fullyRetrieved = i
			}
		})
	}

	function toJob(x: any): Job {
		return x as Job
	}

	function toCompletedJob(x: any): CompletedJob {
		return x as CompletedJob
	}

	function toCompletedJobs(x: any): CompletedJob[] {
		return x as CompletedJob[]
	}

	$: $workspaceStore && job && loadResults()
</script>

<div class="flow-root w-full p-6">
	<div class="flex ">
		{#if job}
			<div class="flex-col">
				<a href="/run/{job?.id}" class="font-medium text-blue-600">
					{truncateRev(job?.id ?? '', 10)}
				</a>
			</div>
		{/if}
	</div>
	<JobStatus {job} />

	<p class="text-gray-500 mb-6 w-full text-center">
		Step
		<span class="font-medium text-gray-900">
			{Math.min((job?.flow_status?.step ?? 0) + 1, job?.raw_flow?.modules.length ?? 0)}
		</span>
		out of <span class="font-medium text-gray-900">{job?.raw_flow?.modules.length}</span>
		<span class="mt-4" />
	</p>

	<ul class="w-full">
		{#each job?.raw_flow?.modules ?? [] as mod, i}
			<li class="w-full">
				<div class="relative w-full">
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
									{#if mod.value.type == 'script'}
										Script at path <a
											target="_blank"
											href={scriptPathToHref(mod.value.path ?? '')}
											class="font-medium text-gray-900">{mod.value.path}</a
										>
									{/if}
								</p>
							</div>
							<div class="text-right text-sm whitespace-nowrap text-gray-500">
								{job.flow_status?.modules[i].type}
								<div class=" max-h-40 overflow-y-auto">
									{#if job.flow_status?.modules[i].forloop_jobs}
										{#each job.flow_status?.modules[i].forloop_jobs ?? [] as job}
											<div class="flex flex-col">
												<a href="/run/{job}" class="font-medium text-blue-600">
													{truncateRev(job ?? '', 10)}
												</a>
											</div>
										{/each}
									{:else if job.flow_status?.modules[i].job}
										<a
											href="/run/{job.flow_status?.modules[i].job}"
											class="font-medium text-blue-600"
										>
											{truncateRev(job.flow_status?.modules[i].job ?? '', 10)}
										</a>
									{/if}
								</div>
							</div>
						</div>
					</div>
					{#if jobs[i]}
						{#if Array.isArray(jobs[i])}
							<div class="flex flex-col mt-2 space-y-2 max-h-60 overflow-y-auto shadow-inner">
								{#each toCompletedJobs(jobs[i]) as job, i}
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
										>Iteration: #{i}: {job.id} {forloop_selected == job.id ? '(-)' : '(+)'}</button
									>
									{#if forloop_selected == job.id}
										<svelte:self {job} />
										{#if `result` in job}
											<FlowJobResult {job} />
										{/if}
									{/if}
								{/each}
							</div>
						{:else if toJob(jobs[i]).type == 'CompletedJob'}
							<FlowJobResult job={toCompletedJob(jobs[i])} />
						{:else if jobs[i]}
							{#if toJob(jobs[i])?.raw_flow}
								<div class="border-2">
									<h2>Forloop current iteration</h2>
									<svelte:self job={jobs[i]} />
								</div>
							{:else}
								<div class="max-w-2xl mt-2 h-full">
									<pre
										bind:this={pres[i]}
										class="break-all p-4 relative h-full mx-2 bg-gray-50 text-xs max-h-40 overflow-y-auto border">{toJob(
											jobs[i]
										).logs ?? ''}
							</pre>
								</div>
							{/if}
						{/if}
					{/if}
				</div>
			</li>
		{/each}
	</ul>
</div>
