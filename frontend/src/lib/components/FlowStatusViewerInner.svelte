<script lang="ts">
	import {
		type FlowStatusModule,
		type Job,
		JobService,
		type FlowStatus,
		type FlowModuleValue,
		type FlowModule
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { onDestroy } from 'svelte'
	import { Badge, Button, Tab } from './common'
	import DisplayResult from './DisplayResult.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import { type DurationStatus, type FlowStatusViewerContext, type GraphModuleState } from './graph'
	import ModuleStatus from './ModuleStatus.svelte'
	import { emptyString, msToSec, truncateRev } from '$lib/utils'
	import JobArgs from './JobArgs.svelte'
	import { ChevronDown, Hourglass, Loader2 } from 'lucide-svelte'
	import FlowStatusWaitingForEvents from './FlowStatusWaitingForEvents.svelte'
	import { deepEqual } from 'fast-equals'
	import FlowTimeline from './FlowTimeline.svelte'
	import { dfs } from './flows/dfs'
	import { writable, type Writable } from 'svelte/store'
	import Alert from './common/alert/Alert.svelte'
	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'

	const dispatch = createEventDispatcher()

	let {
		flowStateStore,
		retryStatus,
		suspendStatus,
		hideDownloadInGraph,
		hideTimeline,
		hideNodeDefinition,
		hideDownloadLogs
	} = getContext<FlowStatusViewerContext>('FlowStatusViewer')

	export let jobId: string
	export let initialJob: Job | undefined = undefined
	export let workspaceId: string | undefined = undefined
	export let flowJobIds:
		| {
				moduleId: string
				flowJobs: string[]
				flowJobsSuccess: (boolean | undefined)[]
				length: number
		  }
		| undefined = undefined

	//only useful when forloops are optimized and the job doesn't contain the mod id anymore
	export let innerModule: FlowModuleValue | undefined = undefined

	export let render = true

	export let isOwner = false

	export let selectedNode: string | undefined = undefined

	export let globalModuleStates: Writable<Record<string, GraphModuleState>>[]
	export let globalDurationStatuses: Writable<Record<string, DurationStatus>>[]
	export let globalRefreshes: Record<
		string,
		(loopJob: { index: number; job: string }) => Promise<void>
	> = {}
	export let childFlow: boolean = false
	export let reducedPolling = false

	export let wideResults = false
	export let hideFlowResult = false

	let jobResults: any[] =
		flowJobIds?.flowJobs?.map((x, id) => `iter #${id + 1} not loaded by frontend yet`) ?? []

	let retry_selected = ''
	let timeout: NodeJS.Timeout

	let localModuleStates: Writable<Record<string, GraphModuleState>> = writable({})
	let localDurationStatuses: Writable<Record<string, DurationStatus>> = writable({})

	export let job: Job | undefined = undefined

	// let lastSize = 0
	// $: {
	// 	let len = (flowJobIds?.flowJobs ?? []).length
	// 	if (len != lastSize) {
	// 		updateForloop(len)
	// 	}
	// }

	function setModuleState(
		key: string,
		value: Partial<GraphModuleState>,
		force?: boolean,
		keepType?: boolean
	) {
		let newValue = { ...($localModuleStates[key] ?? {}), ...value }
		if (!deepEqual($localModuleStates[key], value) || force) {
			;[localModuleStates, ...globalModuleStates].forEach((s) => {
				s.update((x) => {
					if (keepType && (x[key]?.type == 'Success' || x[key]?.type == 'Failure')) {
						newValue.type = x[key].type
					}
					x[key] = newValue
					return x
				})
			})
		}
	}

	function setDurationStatusByJob(key: string, id: string, value: any) {
		if (!deepEqual($localDurationStatuses[key]?.byJob[id], value)) {
			$localDurationStatuses[key].byJob[id] = value
			globalDurationStatuses.forEach((s) => {
				s.update((x) => {
					x[key].byJob[id] = value

					return x
				})
			})
		}
	}

	function initializeByJob(modId: string) {
		if ($localDurationStatuses[modId] == undefined) {
			$localDurationStatuses[modId] = { byJob: {} }
		}
		let prefixed = modId
		globalDurationStatuses.forEach((x) =>
			x.update((x) => {
				if (x[prefixed] == undefined) {
					x[prefixed] = { byJob: {} }
				}
				return x
			})
		)
	}

	let innerModules: FlowStatusModule[] = []

	function updateStatus(status: FlowStatus) {
		innerModules =
			status?.modules?.concat(
				status.failure_module.type != 'WaitingForPriorSteps' ? status.failure_module : []
			) ?? []
		if (status.preprocessor_module) {
			innerModules.unshift(status.preprocessor_module)
		}
		updateInnerModules()

		let count = status.retry?.fail_count
		if (count) {
			$retryStatus[jobId ?? ''] = count
		} else if ($retryStatus[jobId ?? ''] != undefined) {
			delete $retryStatus[jobId ?? '']
			$retryStatus = $retryStatus
		}
		let jobStatus = job?.flow_status?.modules?.[job?.flow_status.step]

		if (jobStatus && jobStatus.count != undefined) {
			$suspendStatus[jobId ?? ''] = { nb: jobStatus.count, job: job! }
		} else if ($suspendStatus[jobId ?? ''] != undefined) {
			delete $suspendStatus[jobId ?? '']
			$suspendStatus = $suspendStatus
		}
	}

	function updateInnerModules() {
		if ($localModuleStates) {
			innerModules.forEach((mod, i) => {
				if (
					mod.type === 'WaitingForEvents' &&
					$localModuleStates?.[innerModules?.[i - 1]?.id ?? '']?.type == 'Success'
				) {
					setModuleState(mod.id ?? '', { type: mod.type, args: job?.args, tag: job?.tag })
				} else if (
					mod.type === 'WaitingForExecutor' &&
					$localModuleStates[mod.id ?? '']?.scheduled_for == undefined
				) {
					JobService.getJob({
						workspace: workspaceId ?? $workspaceStore ?? '',
						id: mod.job ?? '',
						noLogs: true
					})
						.then((job) => {
							const newState = {
								type: mod.type,
								scheduled_for: job?.['scheduled_for'],
								job_id: job?.id,
								parent_module: mod['parent_module'],
								args: job?.args,
								tag: job?.tag
							}

							setModuleState(mod.id ?? '', newState)
						})
						.catch((e) => {
							console.error(`Could not load inner module for job ${mod.job}`, e)
						})
				} else if (
					mod.flow_jobs &&
					(mod.type == 'Success' || mod.type == 'Failure') &&
					!['Success', 'Failure'].includes($localModuleStates?.[mod.id ?? '']?.type)
				) {
					setModuleState(
						mod.id ?? '',
						{
							type: mod.type
						},
						true
					)
				}
				if (mod.branch_chosen) {
					setModuleState(
						mod.id ?? '',
						{
							branchChosen:
								mod.branch_chosen.type == 'default' ? 0 : (mod.branch_chosen.branch ?? 0) + 1
						},
						true
					)
				}

				/**
				 * else if (mod.type === 'Failure' || mod.type === 'WaitingForPriorSteps') {
					if (job?.type === 'CompletedJob') {
						setModuleState('b', {
							type: 'Failure',
							args: job?.args,
							job_id: job?.id,
							result: job?.result
						})
					}
				}
				*/
			})
		}
	}

	let recursiveRefresh: Record<string, (boolean) => Promise<void>> = {}

	export async function refresh(
		root: boolean,
		loopJob: { index: number; job: string } | undefined
	) {
		let modId = flowJobIds?.moduleId

		if (!loopJob) {
			loopJob = {
				index: $localModuleStates[modId ?? '']?.selectedForloopIndex ?? 0,
				job: $localModuleStates[modId ?? '']?.selectedForloop ?? ''
			}
		}

		let last = root ? undefined : flowJobIds?.flowJobs?.[flowJobIds?.flowJobs.length - 1]

		// console.log(innerModule, modId)

		Object.entries(recursiveRefresh).forEach(([key, v]) => {
			if (modId) {
				if ((root && key == loopJob?.job) || key == last) {
					v(false)
				}
			} else {
				v(false)
			}
		})

		let njob = flowJobIds
			? root && modId
				? storedListJobs?.[loopJob.job]
				: storedListJobs?.[flowJobIds.length - 1]
			: job

		if (njob) {
			dispatch('jobsLoaded', { job: njob, force: true })
		}
	}

	let errorCount = 0
	let notAnonynmous = false
	async function loadJobInProgress() {
		dispatch('start')
		if (jobId != '00000000-0000-0000-0000-000000000000') {
			try {
				const newJob =
					jobId == initialJob?.id &&
					initialJob?.id != undefined &&
					initialJob?.type === 'CompletedJob'
						? initialJob
						: await JobService.getJob({
								workspace: workspaceId ?? $workspaceStore ?? '',
								id: jobId ?? '',
								noLogs: true
						  })
				if (!deepEqual(job, newJob)) {
					job = newJob
					job?.flow_status && updateStatus(job?.flow_status)
					dispatch('jobsLoaded', { job, force: false })
				}
				errorCount = 0
				notAnonynmous = false
			} catch (e) {
				if (
					e?.body?.includes('As a non logged in user, you can only see jobs ran by anonymous users')
				) {
					notAnonynmous = true
				} else {
					errorCount += 1
					console.error(e)
				}
			}
		}
		if (job?.type !== 'CompletedJob' && errorCount < 4 && !destroyed) {
			timeout = setTimeout(() => loadJobInProgress(), reducedPolling ? 5000 : 1000)
		} else {
			dispatch('done', job)
		}
	}

	let destroyed = false
	async function updateJobId() {
		if (jobId !== job?.id) {
			$localModuleStates = {}
			flowTimeline?.reset()
			timeout && clearTimeout(timeout)
			innerModules = []
			if (flowJobIds) {
				let modId = flowJobIds?.moduleId ?? ''

				let common = {
					iteration_from:
						// $localDurationStatuses?.[modId]?.iteration_from ??
						Math.max(flowJobIds.flowJobs.length - 20, 0),
					iteration_total: $localDurationStatuses?.[modId]?.iteration_total ?? flowJobIds?.length
				}
				$localDurationStatuses[modId] = {
					...($localDurationStatuses[modId] ?? { byJob: {} }),
					...common
				}
				let prefixed = modId
				globalDurationStatuses.forEach((x) =>
					x.update((x) => {
						x[prefixed] = { ...(x[prefixed] ?? { byJob: {} }), ...common }
						return x
					})
				)
			} else {
				$localDurationStatuses = {}
			}
			await loadJobInProgress()
		}
	}

	$: jobId && updateJobId()

	$: isListJob = flowJobIds != undefined && Array.isArray(flowJobIds?.flowJobs)

	$: flowJobIds?.moduleId && onFlowJobFlowStatus()

	function onFlowJobFlowStatus() {
		if (globalRefreshes) {
			let modId = flowJobIds?.moduleId
			if (modId) {
				globalRefreshes[modId] = async (loopJob) => {
					setIteration(loopJob.index, loopJob.job, false, modId ?? '')
					refresh(true, loopJob)
				}
			}
		}
	}

	onDestroy(() => {
		destroyed = true
		timeout && clearTimeout(timeout)
	})

	$: selected = isListJob ? 'sequence' : 'graph'

	function isSuccess(arg: any): boolean | undefined {
		if (arg == undefined) {
			return undefined
		} else {
			return arg == true
		}
	}

	function onJobsLoaded(mod: FlowStatusModule, job: Job, force?: boolean): void {
		if (mod.id && (mod.flow_jobs ?? []).length == 0) {
			if (!childFlow) {
				if ($flowStateStore?.[mod.id]) {
					$flowStateStore[mod.id] = {
						...$flowStateStore[mod.id],
						previewResult: job['result'],
						previewArgs: job.args
					}
				}
			}
			initializeByJob(mod.id)
			let started_at = job.started_at ? new Date(job.started_at).getTime() : undefined
			if (job.type == 'QueuedJob') {
				setModuleState(
					mod.id,
					{
						type: 'InProgress',
						job_id: job.id,
						logs: job.logs,
						args: job.args,
						tag: job.tag,
						started_at,
						parent_module: mod['parent_module']
					},
					force
				)
				setDurationStatusByJob(mod.id, job.id, {
					created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
					started_at
				})
			} else {
				const parent_module = mod['parent_module']

				// Delete existing failure node attached to the same parent module
				removeFailureNode(mod.id, parent_module)

				setModuleState(
					mod.id,
					{
						args: job.args,
						type: job['success'] ? 'Success' : 'Failure',
						logs: job.logs,
						result: job['result'],
						job_id: job.id,
						tag: job.tag,
						parent_module,
						duration_ms: job['duration_ms'],
						started_at: started_at,
						flow_jobs: mod.flow_jobs,
						flow_jobs_success: mod.flow_jobs_success,
						iteration_total: mod.iterator?.itered?.length,
						retries: mod?.failed_retries?.length,
						skipped: mod.skipped
						// retries: $flowStateStore?.raw_flow
					},
					force
				)

				setDurationStatusByJob(mod.id, job.id, {
					created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
					started_at,
					duration_ms: job['duration_ms']
				})
			}
		}
	}

	function setIteration(j: number, id: string, clicked: boolean, modId: string) {
		if (modId) {
			if (!$localModuleStates?.[modId]) {
				$localModuleStates[modId] = {
					type: 'InProgress',
					args: undefined
				}
			}
			let state = $localModuleStates?.[modId]

			if (state) {
				if (state.selectedForloop == id && clicked) {
					setModuleState(
						modId,
						{
							selectedForloop: undefined,
							selectedForloopIndex: -1
						},
						false,
						true
					)
				} else {
					setModuleState(
						modId,
						{
							selectedForloop: id,
							selectedForloopIndex: j
						},
						false,
						true
					)
					clicked && refresh(true, undefined)
				}
			}
		}
	}
	function innerJobLoaded(jobLoaded: Job, j: number, clicked: boolean, force: boolean) {
		let modId = flowJobIds?.moduleId
		if (modId) {
			setIteration(j, jobLoaded.id, clicked, modId)

			if ($flowStateStore && $flowStateStore?.[modId] == undefined) {
				$flowStateStore[modId] = {
					...(($flowStateStore[modId] as object) ?? {}),
					previewResult: jobLoaded.args
				}
			}
			if ($flowStateStore?.[modId]) {
				if (!childFlow) {
					if (
						!$flowStateStore[modId].previewResult ||
						!Array.isArray($flowStateStore[modId]?.previewResult)
					) {
						$flowStateStore[modId].previewResult = []
					}
					$flowStateStore[modId].previewArgs = jobLoaded.args
				}
				if (jobLoaded.type == 'QueuedJob') {
					jobResults[j] = 'Job in progress ...'
				} else if (jobLoaded.type == 'CompletedJob') {
					$flowStateStore[modId].previewResult[j] = jobLoaded.result
					jobResults[j] = jobLoaded.result
				}
			}

			let started_at = jobLoaded.started_at ? new Date(jobLoaded.started_at).getTime() : undefined

			let created_at = jobLoaded.created_at ? new Date(jobLoaded.created_at).getTime() : undefined

			let job_id = jobLoaded.id
			initializeByJob(modId)

			if (jobLoaded.type == 'QueuedJob') {
				if ($localModuleStates[modId]?.selectedForloopIndex == j) {
					setModuleState(
						modId,
						{
							started_at,
							logs: jobLoaded.logs,
							job_id,
							args: jobLoaded.args,
							flow_jobs: flowJobIds?.flowJobs,
							flow_jobs_success: flowJobIds?.flowJobsSuccess,
							iteration_total: flowJobIds?.length,
							duration_ms: undefined
						},
						force,
						true
					)
				}
				setDurationStatusByJob(modId, job_id, {
					created_at,
					started_at
				})
			} else if (jobLoaded.type == 'CompletedJob') {
				if ($localModuleStates[modId]?.selectedForloopIndex == j) {
					setModuleState(
						modId,
						{
							started_at,
							args: jobLoaded.args,
							result: jobLoaded.result,
							flow_jobs_results: jobResults,
							job_id,
							flow_jobs: flowJobIds?.flowJobs,
							flow_jobs_success: flowJobIds?.flowJobsSuccess,
							iteration_total: flowJobIds?.length,
							duration_ms: undefined,
							isListJob: true
						},
						force,
						true
					)
				}
				setDurationStatusByJob(modId, job_id, {
					created_at,
					started_at,
					duration_ms: jobLoaded.duration_ms
				})
			}

			if (jobLoaded.job_kind == 'script' || jobLoaded.job_kind == 'preview') {
				let id: string | undefined = undefined
				if (innerModule?.type == 'forloopflow' && innerModule.modules.length == 1) {
					id = innerModule?.modules?.[0]?.id
				}
				if (id) {
					onJobsLoaded({ id } as FlowStatusModule, jobLoaded)
				}
			}
		}
	}

	let flowTimeline: FlowTimeline

	let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' = 'timeline'

	function loadPreviousIters(lenToAdd: number) {
		let r = $localDurationStatuses[flowJobIds?.moduleId ?? '']
		if (r.iteration_from) {
			r.iteration_from -= lenToAdd
			$localDurationStatuses = $localDurationStatuses
			globalDurationStatuses.forEach((x) => x.update((x) => x))
		}
		jobResults = [
			...[...new Array(lenToAdd).keys()].map((x) => 'not computed or loaded yet'),
			...jobResults
		]
		// updateSlicedListJobIds()
	}

	let stepDetail: FlowModule | string | undefined = undefined

	let storedListJobs: Record<number, Job> = {}
	let wrapperHeight: number = 0

	function removeFailureNode(id: string, parent_module: any) {
		if (id?.startsWith('failure-') && parent_module) {
			;[...globalModuleStates, localModuleStates].forEach((stateMapStore) => {
				stateMapStore.update((stateMap) => {
					if (id) {
						Object.keys(stateMap).forEach((key) => {
							if (stateMap[key]?.parent_module == parent_module) {
								delete stateMap[key]
							}
						})
					}
					return stateMap
				})
			})
		}
	}
</script>

{#if notAnonynmous}
	<Alert type="error" title="Required Auth">
		As a non logged in user, you can only see jobs ran by anonymous users like you
	</Alert>
{:else if job}
	<div class="flow-root w-full space-y-4 {wideResults ? '' : 'max-w-7xl'} mx-auto px-4">
		<!-- {#if innerModules.length > 0 && true}
			<h3 class="text-md leading-6 font-bold text-primay border-b pb-2">Flow result</h3>
		{:else}
			<div class="h-8" />
		{/if} -->
		{#if isListJob}
			{@const sliceFrom = $localDurationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0}
			{@const lenToAdd = Math.min(20, sliceFrom)}

			{#if (flowJobIds?.flowJobs.length ?? 0) > 20 && lenToAdd > 0}
				{@const allToAdd = (flowJobIds?.length ?? 0) - sliceFrom}
				<p class="text-tertiary italic text-xs">
					For performance reasons, only the last 20 items are shown by default <button
						class="text-primary underline ml-4"
						on:click={() => {
							loadPreviousIters(lenToAdd)
						}}
						>Load {lenToAdd} prior
					</button>
					{#if allToAdd > 0 && allToAdd > lenToAdd}
						{sliceFrom}
						<button
							class="text-primary underline ml-4"
							on:click={() => {
								loadPreviousIters(allToAdd)
							}}
							>Load {allToAdd} prior
						</button>
					{/if}
				</p>
			{/if}
			{#if render}
				<div class="w-full h-full border rounded-sm bg-surface p-1 overflow-auto">
					<DisplayResult workspaceId={job?.workspace_id} {jobId} result={jobResults} />
				</div>
			{/if}
		{:else if render}
			<div class={'border rounded-md shadow p-2'}>
				<FlowPreviewStatus {job} />
				{#if !job}
					<div>
						<Loader2 class="animate-spin" />
					</div>
				{:else if `result` in job}
					{#if !hideFlowResult}
						<div class="w-full h-full">
							<FlowJobResult
								workspaceId={job?.workspace_id}
								jobId={job?.id}
								tag={job?.tag}
								loading={job['running'] == true}
								result={job.result}
								logs={job.logs}
								durationStates={localDurationStatuses}
								downloadLogs={!hideDownloadLogs}
							/>
						</div>
					{/if}
				{:else if job.flow_status?.modules?.[job?.flow_status?.step]?.type === 'WaitingForEvents'}
					<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
				{:else if $suspendStatus && Object.keys($suspendStatus).length > 0}
					<div class="flex gap-2 flex-col">
						{#each Object.values($suspendStatus) as suspendCount (suspendCount.job.id)}
							<div>
								<div class="text-sm">
									Flow suspended, waiting for {suspendCount.nb} events
								</div>
								<FlowStatusWaitingForEvents job={suspendCount.job} {workspaceId} {isOwner} />
							</div>
						{/each}
					</div>
				{:else if job.logs}
					<div
						class="text-xs p-4 bg-surface-secondary overflow-auto max-h-80 border border-tertiary-inverse"
					>
						<pre class="w-full">{job.logs}</pre>
					</div>
				{:else if innerModules?.length > 0}
					<div class="flex flex-col gap-1">
						{#each innerModules as mod, i (mod.id)}
							{#if mod.type == 'InProgress'}
								{@const rawMod = job.raw_flow?.modules[i]}

								<div
									><span class="inline-flex gap-1"
										><Badge color="indigo">{mod.id}</Badge>
										<span class="font-medium text-primary">
											{#if !emptyString(rawMod?.summary)}
												{rawMod?.summary ?? ''}
											{:else if rawMod?.value.type == 'script'}
												{rawMod.value.path ?? ''}
											{:else if rawMod?.value.type}
												{rawMod?.value.type}
											{/if}
										</span>

										<Loader2 class="animate-spin" /></span
									></div
								>
							{/if}
						{/each}
					</div>
				{/if}
			</div>
		{/if}
		{#if render}
			{#if innerModules.length > 0 && !isListJob}
				<Tabs class="mx-auto {wideResults ? '' : 'max-w-7xl'}" bind:selected>
					<Tab value="graph"><span class="font-semibold text-md">Graph</span></Tab>
					<Tab value="sequence"><span class="font-semibold">Details</span></Tab>
				</Tabs>
			{/if}
		{/if}
		<div class="{selected != 'sequence' ? 'hidden' : ''} max-w-7xl mx-auto">
			{#if isListJob}
				{@const sliceFrom = $localDurationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0}
				{@const forloop_selected =
					$localModuleStates?.[flowJobIds?.moduleId ?? '']?.selectedForloop}
				<h3 class="text-md leading-6 font-bold text-tertiary border-b mb-4">
					Subflows ({flowJobIds?.flowJobs.length})
				</h3>
				<div class="overflow-auto max-h-1/2">
					{#each flowJobIds?.flowJobs ?? [] as loopJobId, j (loopJobId)}
						{#if render}
							<Button
								variant={forloop_selected === loopJobId ? 'contained' : 'border'}
								color={flowJobIds?.flowJobsSuccess?.[j] === false
									? 'red'
									: forloop_selected === loopJobId
									? 'dark'
									: 'light'}
								btnClasses="w-full flex justify-start"
								on:click={async () => {
									let storedJob = storedListJobs[j]
									if (!storedJob) {
										storedJob = await JobService.getJob({
											workspace: workspaceId ?? $workspaceStore ?? '',
											id: loopJobId,
											noLogs: true
										})
										storedListJobs[j] = storedJob
									}
									innerJobLoaded(storedJob, j, true, false)
								}}
								endIcon={{
									icon: ChevronDown,
									classes: forloop_selected == loopJobId ? '!rotate-180' : ''
								}}
							>
								<span class="truncate font-mono">
									#{j + 1}: {loopJobId}
								</span>
							</Button>
						{/if}
						{#if j >= sliceFrom || forloop_selected == loopJobId}
							<!-- <LogId id={loopJobId} /> -->
							<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
								<svelte:self
									bind:refresh={recursiveRefresh[loopJobId]}
									{globalRefreshes}
									{childFlow}
									job={storedListJobs[j]}
									globalModuleStates={[localModuleStates, ...globalModuleStates]}
									globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
									render={forloop_selected == loopJobId && selected == 'sequence' && render}
									reducedPolling={flowJobIds?.flowJobs.length && flowJobIds?.flowJobs.length > 20}
									{workspaceId}
									jobId={loopJobId}
									on:jobsLoaded={(e) => {
										let { job, force } = e.detail
										storedListJobs[j] = job
										innerJobLoaded(job, j, false, force)
									}}
								/>
							</div>
						{/if}
					{/each}
				</div>
			{:else if innerModules.length > 0 && (job.raw_flow?.modules.length ?? 0) > 0}
				{@const hasPreprocessor = innerModules[0]?.id == 'preprocessor' ? 1 : 0}
				<ul class="w-full">
					<h3 class="text-md leading-6 font-bold text-primary border-b mb-4 py-2">
						Step-by-step
					</h3>

					{#each innerModules as mod, i}
						{#if render}
							<div class="line w-8 h-10" />
							<h3 class="text-tertiary mb-2 w-full">
								{#if mod.id === 'preprocessor'}
									<h3>Preprocessor module</h3>
								{:else if job?.raw_flow?.modules && i < job?.raw_flow?.modules.length + hasPreprocessor}
									Step
									<span class="font-medium text-primary">
										{i + 1 - hasPreprocessor}
									</span>
									out of
									<span class="font-medium text-primary">{job?.raw_flow?.modules.length}</span>
									{#if job.raw_flow?.modules[i]?.summary}
										: <span class="font-medium text-primary">
											{job.raw_flow?.modules[i]?.summary ?? ''}
										</span>
									{/if}
								{:else}
									<h3>Failure module</h3>
								{/if}
							</h3>
							<div class="line w-8 h-10" />
						{/if}
						<li class="w-full border p-6 space-y-2 bg-blue-50/50 dark:bg-frost-900/50">
							{#if render && Array.isArray(mod.failed_retries)}
								{#each mod.failed_retries as failedRetry, j}
									<Button
										variant={retry_selected === failedRetry ? 'contained' : 'border'}
										color="red"
										btnClasses="w-full flex justify-start"
										on:click={() => {
											if (retry_selected == failedRetry) {
												retry_selected = ''
											} else {
												retry_selected = failedRetry
											}
										}}
										endIcon={{
											icon: ChevronDown,
											classes: retry_selected == failedRetry ? '!rotate-180' : ''
										}}
									>
										<span class="truncate font-mono">
											# Attempt {j + 1}: {failedRetry}
										</span>
									</Button>

									<!-- <LogId id={loopJobId} /> -->
									<div class="border p-6" class:hidden={retry_selected != failedRetry}>
										<svelte:self
											{globalRefreshes}
											bind:refresh={recursiveRefresh[failedRetry]}
											{childFlow}
											globalModuleStates={[localModuleStates, ...globalModuleStates]}
											globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
											render={failedRetry == retry_selected && render}
											reducedPolling={false}
											{workspaceId}
											jobId={failedRetry}
										/>
									</div>
								{/each}
							{/if}
							{#if ['InProgress', 'Success', 'Failure'].includes(mod.type)}
								{#if job.raw_flow?.modules[i]?.value.type == 'flow'}
									<svelte:self
										{globalRefreshes}
										bind:refresh={recursiveRefresh[mod.job ?? '']}
										globalModuleStates={[]}
										globalDurationStatuses={[]}
										render={selected == 'sequence' && render}
										{workspaceId}
										jobId={mod.job}
										childFlow
										on:jobsLoaded={(e) => {
											let { force, job } = e.detail
											onJobsLoaded(mod, job, force)
										}}
									/>
								{:else if mod.flow_jobs?.length == 0 && mod.job == '00000000-0000-0000-0000-000000000000'}
									<div class="text-secondary">no subflow (empty loop?)</div>
								{:else}
									<svelte:self
										{globalRefreshes}
										bind:refresh={recursiveRefresh[mod.job ?? '']}
										{childFlow}
										globalModuleStates={[localModuleStates, ...globalModuleStates]}
										globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
										render={selected == 'sequence' && render}
										{workspaceId}
										jobId={mod.job}
										innerModule={mod.flow_jobs ? job.raw_flow?.modules[i]?.value : undefined}
										flowJobIds={mod.flow_jobs
											? {
													moduleId: mod.id,
													flowJobs: mod.flow_jobs,
													flowJobsSuccess: mod.flow_jobs_success,
													length: mod.iterator?.itered?.length ?? mod.flow_jobs.length
											  }
											: undefined}
										on:jobsLoaded={(e) => {
											let { job, force } = e.detail
											onJobsLoaded(mod, job, force)
										}}
									/>
								{/if}
							{:else}
								<ModuleStatus
									type={mod.type}
									scheduled_for={$localModuleStates?.[mod.id ?? '']?.scheduled_for}
								/>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<div class="p-2 text-tertiary text-sm italic">Empty flow</div>
			{/if}
		</div>
	</div>
	{#if render}
		{#if job.raw_flow && !isListJob}
			<div class="{selected != 'graph' ? 'hidden' : ''} grow mt-4">
				<div class="grid grid-cols-3 border h-full" bind:clientHeight={wrapperHeight}>
					<div class="col-span-2 bg-surface-secondary">
						<div class="flex flex-col">
							{#each Object.values($retryStatus) as count}
								{#if count}
									<span class="text-sm">
										Retry in progress, # of failed attempts: {count}
									</span>
								{/if}
							{/each}
							{#each Object.values($suspendStatus) as count}
								{#if count.nb}
									<span class="text-sm">
										Flow suspended, waiting for {count.nb} events
									</span>
								{/if}
							{/each}
						</div>

						<FlowGraphV2
							triggerNode={true}
							download={!hideDownloadInGraph}
							minHeight={wrapperHeight}
							success={jobId != undefined && isSuccess(job?.['success'])}
							flowModuleStates={$localModuleStates}
							on:select={(e) => {
								if (rightColumnSelect != 'node_definition') {
									rightColumnSelect = 'node_status'
								}
								if (typeof e.detail == 'string') {
									if (e.detail == 'Input') {
										selectedNode = 'start'
										stepDetail = undefined
									} else if (e.detail == 'Result') {
										selectedNode = 'end'
										stepDetail = 'end'
									} else {
										const mod = dfs(job?.raw_flow?.modules ?? [], (m) => m).find(
											(m) => m?.id === e?.detail
										)
										stepDetail = mod
										selectedNode = e?.detail
									}
								} else {
									stepDetail = e.detail
									selectedNode = e.detail.id
								}
							}}
							on:selectedIteration={(e) => {
								let detail = e.detail

								setModuleState(detail.moduleId, {
									selectedForloop: detail.id,
									selectedForloopIndex: detail.index
								})
								globalRefreshes[detail.moduleId]?.({ job: detail.id, index: detail.index })
							}}
							modules={job.raw_flow?.modules ?? []}
							failureModule={job.raw_flow?.failure_module}
							preprocessorModule={job.raw_flow?.preprocessor_module}
						/>
					</div>
					<div
						class="border-l border-tertiary-inverse pt-1 overflow-auto min-h-[700px] flex flex-col z-0 h-full"
					>
						<Tabs bind:selected={rightColumnSelect}>
							{#if !hideTimeline}
								<Tab value="timeline"><span class="font-semibold text-md">Timeline</span></Tab>
							{/if}
							<Tab value="node_status"><span class="font-semibold">Node status</span></Tab>
							{#if !hideNodeDefinition}
								<Tab value="node_definition"><span class="font-semibold">Node definition</span></Tab
								>
							{/if}
							{#if Object.keys(job?.flow_status?.user_states ?? {}).length > 0}
								<Tab value="user_states"><span class="font-semibold">User States</span></Tab>
							{/if}
						</Tabs>
						{#if rightColumnSelect == 'timeline'}
							<FlowTimeline
								selfWaitTime={job?.self_wait_time_ms}
								aggregateWaitTime={job?.aggregate_wait_time_ms}
								flowDone={job?.['success'] != undefined}
								bind:this={flowTimeline}
								flowModules={dfs(job.raw_flow?.modules ?? [], (x) => x.id)}
								durationStatuses={localDurationStatuses}
							/>
						{:else if rightColumnSelect == 'node_status'}
							<div class="pt-2 max-h-[80vh] grow flex flex-col">
								{#if selectedNode}
									{@const node = $localModuleStates[selectedNode]}

									{#if selectedNode == 'end'}
										<FlowJobResult
											workspaceId={job?.workspace_id}
											jobId={job?.id}
											filename={job.id}
											loading={job['running']}
											tag={job?.tag}
											noBorder
											col
											result={job['result']}
											logs={job.logs ?? ''}
											durationStates={localDurationStatuses}
											downloadLogs={!hideDownloadLogs}
										/>
									{:else if selectedNode == 'start'}
										{#if job.args}
											<div class="p-2">
												<JobArgs
													id={job.id}
													workspace={job.workspace_id ?? $workspaceStore ?? 'no_w'}
													args={job.args}
												/>
											</div>
										{:else}
											<p class="p-2 text-secondary">No arguments</p>
										{/if}
									{:else if node}
										{#if node.flow_jobs_results}
											<span class="pl-1 text-tertiary"
												>Result of step as collection of all subflows</span
											>
											<div class="overflow-auto max-h-[200px] p-2">
												<DisplayResult
													workspaceId={job?.workspace_id}
													result={node.flow_jobs_results}
													nodeId={selectedNode}
													jobId={job?.id}
												/>
											</div>
											<span class="pl-1 text-tertiary text-lg pt-4">Selected subflow</span>
										{/if}
										<div class="px-2 flex gap-2 min-w-0 w-full">
											<ModuleStatus
												type={node.type}
												scheduled_for={node.scheduled_for}
												skipped={node.skipped}
											/>
											{#if node.duration_ms}
												<Badge>
													<Hourglass class="mr-2" size={10} />
													{msToSec(node.duration_ms)} s
												</Badge>
											{/if}
											{#if node.job_id}
												<div class="grow w-full flex flex-row-reverse">
													<a
														class="text-right text-xs"
														rel="noreferrer"
														target="_blank"
														href="{base}/run/{node.job_id ?? ''}?workspace={job?.workspace_id}"
													>
														{truncateRev(node.job_id ?? '', 10)}
													</a>
												</div>
											{/if}
										</div>
										{#if !node.isListJob}
											<div class="px-1 py-1">
												<JobArgs
													id={node.job_id}
													workspace={job.workspace_id ?? $workspaceStore ?? 'no_w'}
													args={node.args}
												/>
											</div>
										{/if}

										<FlowJobResult
											workspaceId={job?.workspace_id}
											jobId={node.job_id}
											noBorder
											loading={node.type != 'Success' && node.type != 'Failure'}
											waitingForExecutor={node.type == 'WaitingForExecutor'}
											refreshLog={node.type == 'InProgress'}
											col
											result={node.result}
											tag={node.tag}
											logs={node.logs}
											durationStates={localDurationStatuses}
											downloadLogs={!hideDownloadLogs}
										/>
									{:else}
										<p class="p-2 text-tertiary italic"
											>The execution of this node has no information attached to it. The job likely
											did not run yet</p
										>
									{/if}
								{:else}<p class="p-2 text-tertiary italic">Select a node to see its details here</p
									>{/if}
							</div>
						{:else if rightColumnSelect == 'node_definition'}
							<FlowGraphViewerStep {stepDetail} />
						{:else if rightColumnSelect == 'user_states'}
							<div class="p-2">
								<JobArgs argLabel="Key" args={job?.flow_status?.user_states ?? {}} />
							</div>
						{/if}
					</div>
				</div>
			</div>
		{/if}
	{/if}
{:else}
	<Loader2 class="animate-spin" />
{/if}

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
