<script lang="ts">
	import { JobService } from '$lib/gen'
	import { arePreviewsReady, workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import { Button } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import type { JobResult } from './flows/flowStateUtils'
	import { onMount } from 'svelte'
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
	let isReadyIndex: number | undefined = undefined

	function shouldReset() {
		if (jobId != lastJobid) {
			lastJobid = jobId
			jobResult = {
				job: undefined,
				innerJobs: [],
				loopJobs: []
			}
			loadJobInProgress()
		}
	}

	let lastJobid = ''

	$: jobId && shouldReset()

	async function loadJobInProgress() {
		if (!isReadyIndex) {
			isReadyIndex = $arePreviewsReady.push(false)
		}

		const job = await JobService.getJob({
			workspace: $workspaceStore ?? '',
			id: jobId ?? ''
		})

		jobResult.job = job
		jobResult = jobResult

		if (job?.type === 'CompletedJob') {
			arePreviewsReady.update((isReady: boolean[]) => {
				if (isReadyIndex && isReady[isReadyIndex - 1] === false) {
					isReady[isReadyIndex - 1] = true
				}

				if (root && isReady.every(Boolean)) {
					dispatch('jobsLoaded', jobResult)
					return []
				} else {
					return isReady
				}
			})
		} else {
			setTimeout(() => loadJobInProgress(), 500)
		}
	}

	$: job = jobResult.job
	$: innerJobs = jobResult.innerJobs
	$: loopJobs = jobResult.loopJobs

	$: hasModules = job && Array.isArray(job?.raw_flow?.modules) && job?.raw_flow?.modules.length! > 1

	onMount(() => {
		loadJobInProgress()
	})
</script>

{#if job?.logs}
	<div class="flow-root w-full space-y-4">
		<h3 class="text-md leading-6 font-bold text-gray-900 border-b pb-2">Preview results</h3>
		<FlowPreviewStatus {job} />
		{#if `result` in job}
			<FlowJobResult {job} />
		{:else if job.logs}
			<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
				<pre class="w-full">{job.logs}</pre>
			</div>
		{/if}

		{#if Array.isArray(forloopJobIds) && forloopJobIds?.length > 0 && Array.isArray(loopJobs)}
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
					<svelte:self jobId={loopJobId} bind:jobResult={loopJobs[j]} />
				</div>
			{/each}
		{:else if hasModules && Array.isArray(innerJobs)}
			<ul class="w-full">
				<h3 class="text-md leading-6 font-bold text-gray-900 border-b mb-4 py-2">
					Detailed results
				</h3>

				{#each job?.flow_status?.modules ?? [] as module, i}
					<p class="text-gray-500 mb-6 w-full ">
						Step
						<span class="font-medium text-gray-900"> {i + 1} </span> out of
						<span class="font-medium text-gray-900">{job?.raw_flow?.modules.length}</span>
					</p>

					{#if ['InProgress', 'Success', 'Error'].includes(module.type)}
						<li class="w-full border p-6 space-y-2">
							<svelte:self
								jobId={module.job}
								bind:jobResult={innerJobs[i]}
								forloopJobIds={module.forloop_jobs}
							/>
						</li>
					{/if}
				{/each}
			</ul>
		{/if}
	</div>
{:else}
	No script selected
{/if}
