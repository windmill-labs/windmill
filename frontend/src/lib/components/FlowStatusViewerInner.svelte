<script lang="ts">
	import {
		FlowStatusModule,
		Job,
		JobService,
		type FlowStatus,
		CompletedJob,
		QueuedJob,
		type FlowModuleValue
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { onDestroy } from 'svelte'
	import { Badge, Button, Tab } from './common'
	import DisplayResult from './DisplayResult.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import { FlowGraph, type FlowStatusViewerContext } from './graph'
	import ModuleStatus from './ModuleStatus.svelte'
	import { emptyString, msToSec, truncateRev } from '$lib/utils'
	import JobArgs from './JobArgs.svelte'
	import { ChevronDown, Hourglass, Loader2 } from 'lucide-svelte'
	import FlowStatusWaitingForEvents from './FlowStatusWaitingForEvents.svelte'
	import { deepEqual } from 'fast-equals'
	import FlowTimeline from './FlowTimeline.svelte'
	import { dfs } from './flows/dfs'

	const dispatch = createEventDispatcher()

	let { flowStateStore, flowModuleStates, retryStatus, suspendStatus, durationStatuses } =
		getContext<FlowStatusViewerContext>('FlowStatusViewer')

	export let jobId: string
	export let workspaceId: string | undefined = undefined
	export let flowJobIds:
		| {
				moduleId: string
				flowJobs: string[]
				length: number
		  }
		| undefined = undefined
	export let job: Job | undefined = undefined

	//only useful when forloops are optimized and the job doesn't contain the mod id anymore
	export let innerModule: FlowModuleValue | undefined = undefined

	export let render = true

	export let isOwner = false

	export let selectedNode: string | undefined = undefined

	let jobResults: any[] = []
	let jobFailures: boolean[] = []

	let forloop_selected = ''
	let timeout: NodeJS.Timeout

	let lastSize = 0
	$: {
		let len = (flowJobIds?.flowJobs ?? []).length
		if (len != lastSize) {
			updateForloop(len)
		}
	}

	if (flowJobIds) {
		$durationStatuses[flowJobIds?.moduleId ?? ''] = {
			...($durationStatuses[flowJobIds?.moduleId ?? ''] ?? {}),
			iteration_from: Math.max(flowJobIds.flowJobs.length - 20, 0),
			iteration_total: flowJobIds?.length,
			byJob: {}
		}
	}

	function updateForloop(len: number) {
		forloop_selected = flowJobIds?.flowJobs[len - 1] ?? ''
		lastSize = len
	}

	let innerModules: FlowStatusModule[] = []

	function updateStatus(status: FlowStatus) {
		innerModules =
			status?.modules?.concat(
				status.failure_module.type != 'WaitingForPriorSteps' ? status.failure_module : []
			) ?? []
		updateInnerModules()

		let count = status.retry?.fail_count
		if (count) {
			$retryStatus[jobId ?? ''] = count
		} else if ($retryStatus[jobId ?? ''] != undefined) {
			delete $retryStatus[jobId ?? '']
		}
		$suspendStatus[jobId ?? ''] = job?.flow_status?.modules?.[job?.flow_status.step]?.count
	}

	function updateInnerModules() {
		if ($flowModuleStates) {
			innerModules.forEach((mod, i) => {
				if (
					mod.type === FlowStatusModule.type.WAITING_FOR_EVENTS &&
					$flowModuleStates?.[innerModules?.[i - 1]?.id ?? '']?.type ==
						FlowStatusModule.type.SUCCESS
				) {
					$flowModuleStates[mod.id ?? ''] = { type: mod.type, args: job?.args }
				} else if (
					mod.type === FlowStatusModule.type.WAITING_FOR_EXECUTOR &&
					$flowModuleStates[mod.id ?? '']?.scheduled_for == undefined
				) {
					JobService.getJob({
						workspace: workspaceId ?? $workspaceStore ?? '',
						id: mod.job ?? ''
					})
						.then((job) => {
							const newState = {
								type: mod.type,
								scheduled_for: job?.['scheduled_for'],
								job_id: job?.id,
								parent_module: mod['parent_module'],
								args: job?.args
							}
							if (!deepEqual(newState, $flowModuleStates[mod.id ?? ''])) {
								$flowModuleStates[mod.id ?? ''] = newState
							}
						})
						.catch((e) => {
							console.error(`Could not load inner module for job ${mod.job}`, e)
						})
				}
			})
		}
	}

	let errorCount = 0
	async function loadJobInProgress() {
		if (jobId != '00000000-0000-0000-0000-000000000000') {
			try {
				const newJob = await JobService.getJob({
					workspace: workspaceId ?? $workspaceStore ?? '',
					id: jobId ?? ''
				})
				if (!deepEqual(job, newJob)) {
					job = newJob
					job?.flow_status && updateStatus(job?.flow_status)
					dispatch('jobsLoaded', job)
				}
				errorCount = 0
			} catch (e) {
				errorCount += 1
				console.error(e)
			}
		}
		if (job?.type !== 'CompletedJob' && errorCount < 4) {
			timeout = setTimeout(() => loadJobInProgress(), 500)
		}
	}

	async function updateJobId() {
		if (jobId !== job?.id) {
			flowTimeline?.reset()
			timeout && clearTimeout(timeout)
			innerModules = []
			await loadJobInProgress()
		}
	}

	$: jobId && updateJobId()

	$: isListJob = flowJobIds != undefined && Array.isArray(flowJobIds?.flowJobs)

	onDestroy(() => {
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

	function onJobsLoaded(mod: FlowStatusModule, job: Job): void {
		if (mod.id && (mod.flow_jobs ?? []).length == 0) {
			if ($flowStateStore?.[mod.id]) {
				$flowStateStore[mod.id] = {
					...$flowStateStore[mod.id],
					previewResult: job['result'],
					previewArgs: job.args
				}
			}
			if ($durationStatuses[mod.id] == undefined) {
				$durationStatuses[mod.id] = { byJob: {} }
			}
			let started_at = job.started_at ? new Date(job.started_at).getTime() : undefined
			if (job.type == 'QueuedJob') {
				$flowModuleStates[mod.id] = {
					type: FlowStatusModule.type.IN_PROGRESS,
					job_id: job.id,
					logs: job.logs,
					args: job.args,
					started_at,
					parent_module: mod['parent_module']
				}
				$durationStatuses[mod.id].byJob[job.id] = {
					created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
					started_at
				}
			} else {
				$flowModuleStates[mod.id] = {
					args: job.args,
					type: job['success'] ? FlowStatusModule.type.SUCCESS : FlowStatusModule.type.FAILURE,
					logs: job.logs,
					result: job['result'],
					job_id: job.id,
					parent_module: mod['parent_module'],
					duration_ms: job['duration_ms'],
					started_at: started_at,
					iteration: mod.iterator?.itered?.length,
					iteration_total: mod.iterator?.itered?.length
					// retries: $flowStateStore?.raw_flow
				}
				$durationStatuses[mod.id].byJob[job.id] = {
					created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
					started_at,
					duration_ms: job['duration_ms']
				}
			}
		}
	}

	function innerJobLoaded(
		jobLoaded: (QueuedJob & { type: 'QueuedJob' }) | (CompletedJob & { type: 'CompletedJob' }),
		j: number
	) {
		let modId = flowJobIds?.moduleId
		if (modId) {
			if ($flowStateStore?.[modId]) {
				if (
					!$flowStateStore[modId].previewResult ||
					!Array.isArray($flowStateStore[modId]?.previewResult)
				) {
					$flowStateStore[modId].previewResult = []
				}
				$flowStateStore[modId].previewArgs = jobLoaded.args
				if (jobLoaded.type == 'QueuedJob') {
					jobResults[j] = 'Job in progress ...'
				} else {
					$flowStateStore[modId].previewResult[j] = jobLoaded.result
					jobResults[j] = jobLoaded.result
					jobFailures[j] = jobLoaded.success === false
				}
			}

			let started_at = jobLoaded.started_at ? new Date(jobLoaded.started_at).getTime() : undefined

			let created_at = jobLoaded.created_at ? new Date(jobLoaded.created_at).getTime() : undefined

			let job_id = jobLoaded.id
			if ($durationStatuses[modId] == undefined) {
				$durationStatuses[modId] = { byJob: {} }
			}
			if (jobLoaded.type == 'QueuedJob') {
				$flowModuleStates[modId] = {
					type: FlowStatusModule.type.IN_PROGRESS,
					started_at,
					logs: jobLoaded.logs,
					job_id,
					args: jobLoaded.args,
					iteration: flowJobIds?.flowJobs.length,
					iteration_total: flowJobIds?.length,
					duration_ms: undefined
				}

				$durationStatuses[modId].byJob[job_id] = {
					created_at,
					started_at
				}
			} else {
				$flowModuleStates[modId] = {
					started_at,
					args: jobLoaded.args,
					type: jobLoaded.success ? FlowStatusModule.type.SUCCESS : FlowStatusModule.type.FAILURE,
					logs: 'All jobs completed',
					result: jobResults,
					job_id,
					iteration: flowJobIds?.flowJobs.length,
					iteration_total: flowJobIds?.length,
					duration_ms: undefined,
					isListJob: true
				}
				$durationStatuses[modId].byJob[job_id] = {
					created_at,
					started_at,
					duration_ms: jobLoaded.duration_ms
				}
			}

			if (jobLoaded.job_kind == 'script' || jobLoaded.job_kind == 'preview') {
				let id: string | undefined = undefined
				if (innerModule?.type == 'forloopflow') {
					id = innerModule?.modules?.[0]?.id
				}
				if (id) {
					$flowModuleStates[id] = {
						...($flowModuleStates[modId] ?? {}),
						iteration: undefined,
						isListJob: false,
						iteration_total: undefined
					}
					if ($durationStatuses[id] == undefined) {
						$durationStatuses[id] = { byJob: {} }
					}
					$durationStatuses[id].byJob[job_id] = $durationStatuses[modId].byJob[job_id]
				}
			}
		}
	}

	let flowTimeline: FlowTimeline

	let rightColumnSelect: 'timeline' | 'detail' = 'timeline'
</script>

{#if job}
	<div class="flow-root w-full space-y-4">
		<!-- {#if innerModules.length > 0 && true}
			<h3 class="text-md leading-6 font-bold text-primay border-b pb-2">Flow result</h3>
		{:else}
			<div class="h-8" />
		{/if} -->
		{#if isListJob}
			{@const lenToAdd = Math.min(
				20,
				$durationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0
			)}

			{#if (flowJobIds?.flowJobs.length ?? 0) > 20 && lenToAdd > 0}
				<p class="text-tertiary italic text-xs">
					For performance reasons, only the last 20 items are shown by default <button
						class="text-primary underline ml-4"
						on:click={() => {
							let r = $durationStatuses[flowJobIds?.moduleId ?? '']
							if (r.iteration_from) {
								r.iteration_from -= lenToAdd
								$durationStatuses = $durationStatuses
							}
						}}
						>Load {lenToAdd} prior
					</button>
				</p>
			{/if}
			{#if render}
				<div class="w-full h-full border border-gray-600 bg-surface p-1 overflow-auto">
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
					<div class="w-full h-full">
						<FlowJobResult
							workspaceId={job?.workspace_id}
							jobId={job?.id}
							loading={job['running'] == true}
							result={job.result}
							logs={job.logs ?? ''}
						/>
					</div>
				{:else if job.flow_status?.modules?.[job?.flow_status?.step]?.type === FlowStatusModule.type.WAITING_FOR_EVENTS}
					<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
				{:else if job.logs}
					<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
						<pre class="w-full">{job.logs}</pre>
					</div>
				{:else if innerModules?.length > 0}
					<div class="flex flex-col gap-1">
						{#each innerModules as mod, i (mod.id)}
							{#if mod.type == FlowStatusModule.type.IN_PROGRESS}
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
				<Tabs bind:selected>
					<Tab value="graph"><span class="font-semibold text-md">Graph</span></Tab>
					<Tab value="sequence"><span class="font-semibold">Details</span></Tab>
				</Tabs>
			{/if}
		{/if}
		<div class={selected != 'sequence' ? 'hidden' : ''}>
			{#if isListJob}
				{@const lenToAdd = Math.min(
					20,
					$durationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0
				)}
				<h3 class="text-md leading-6 font-bold text-tertiary border-b mb-4">
					Embedded flows: ({flowJobIds?.flowJobs.length} items)
				</h3>
				{#if (flowJobIds?.flowJobs.length ?? 0) > 20 && lenToAdd > 0}
					<p class="text-tertiary italic text-xs">
						For performance reasons, only the last 20 items are shown by default <button
							class="text-primary underline ml-4"
							on:click={() => {
								let r = $durationStatuses[flowJobIds?.moduleId ?? '']
								if (r.iteration_from) {
									r.iteration_from -= lenToAdd
									$durationStatuses = $durationStatuses
								}
							}}
							>Load {lenToAdd} prior
						</button>
					</p>
				{/if}
				{#each (flowJobIds?.flowJobs.length ?? 0) > 20 ? flowJobIds?.flowJobs?.slice($durationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0) ?? [] : flowJobIds?.flowJobs ?? [] as loopJobId, j (loopJobId)}
					{#if render}
						<Button
							variant={forloop_selected === loopJobId ? 'contained' : 'border'}
							color={jobFailures[j] === true
								? 'red'
								: forloop_selected === loopJobId
								? 'dark'
								: 'light'}
							btnClasses="w-full flex justify-start"
							on:click={() => {
								if (forloop_selected == loopJobId) {
									forloop_selected = ''
								} else {
									forloop_selected = loopJobId
								}
							}}
							endIcon={{
								icon: ChevronDown,
								classes: forloop_selected == loopJobId ? '!rotate-180' : ''
							}}
						>
							<span class="truncate font-mono">
								#{($durationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0) + j + 1}: {loopJobId}
							</span>
						</Button>
					{/if}
					<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
						<svelte:self
							render={forloop_selected == loopJobId && selected == 'sequence' && render}
							{workspaceId}
							jobId={loopJobId}
							on:jobsLoaded={(e) => innerJobLoaded(e.detail, j)}
						/>
					</div>
				{/each}
			{:else if innerModules.length > 0}
				<ul class="w-full">
					<h3 class="text-md leading-6 font-bold text-primary border-b mb-4 py-2">
						Step-by-step
					</h3>

					{#each innerModules as mod, i}
						{#if render}
							<div class="line w-8 h-10" />
							<h3 class="text-tertiary mb-2 w-full">
								{#if job?.raw_flow?.modules && i < job?.raw_flow?.modules.length}
									Step
									<span class="font-medium text-primary">
										{i + 1}
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
							{#if [FlowStatusModule.type.IN_PROGRESS, FlowStatusModule.type.SUCCESS, FlowStatusModule.type.FAILURE].includes(mod.type)}
								{#if job.raw_flow?.modules[i]?.value.type == 'flow'}
									<svelte:self
										render={selected == 'sequence' && render}
										{workspaceId}
										jobId={mod.job}
										on:jobsLoaded={(e) => onJobsLoaded(mod, e.detail)}
									/>
								{:else}
									<svelte:self
										render={selected == 'sequence' && render}
										{workspaceId}
										jobId={mod.job}
										innerModule={mod.flow_jobs ? job.raw_flow?.modules[i]?.value : undefined}
										flowJobIds={mod.flow_jobs
											? {
													moduleId: mod.id,
													flowJobs: mod.flow_jobs,
													length: mod.iterator?.itered?.length ?? mod.flow_jobs.length
											  }
											: undefined}
										on:jobsLoaded={(e) => onJobsLoaded(mod, e.detail)}
									/>
								{/if}
							{:else}
								<ModuleStatus
									type={mod.type}
									scheduled_for={$flowModuleStates?.[mod.id ?? '']?.scheduled_for}
								/>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
	{#if render}
		{#if job.raw_flow && !isListJob}
			<div class="{selected != 'graph' ? 'hidden' : ''} mt-4">
				<div class="grid grid-cols-3 border">
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
								{#if count}
									<span class="text-sm">
										Flow suspended, waiting for {count} events
									</span>
								{/if}
							{/each}
						</div>

						<FlowGraph
							download
							success={isSuccess(job?.['success'])}
							flowModuleStates={$flowModuleStates}
							on:select={(e) => {
								rightColumnSelect = 'detail'
								if (typeof e.detail == 'string') {
									if (e.detail == 'Input') {
										selectedNode = 'start'
									} else if (e.detail == 'Result') {
										selectedNode = 'end'
									} else {
										selectedNode = e.detail
									}
								} else {
									selectedNode = e.detail.id
								}
							}}
							modules={job.raw_flow?.modules ?? []}
							failureModule={job.raw_flow?.failure_module}
						/>
					</div>
					<div class="border-l border-gray-400 pt-1 overflow-auto min-h-[800px] flex flex-col">
						<Tabs bind:selected={rightColumnSelect}>
							<Tab value="timeline"><span class="font-semibold text-md">Timeline</span></Tab>
							<Tab value="detail"><span class="font-semibold">Details</span></Tab>
						</Tabs>
						{#if rightColumnSelect == 'timeline'}
							<FlowTimeline
								flowDone={job?.['success'] != undefined}
								bind:this={flowTimeline}
								flowModules={dfs(job.raw_flow?.modules ?? [], (x) => x.id)}
								{durationStatuses}
							/>
						{:else if rightColumnSelect == 'detail'}
							<div class="pt-2">
								{#if selectedNode}
									{@const node = $flowModuleStates[selectedNode]}

									{#if selectedNode == 'end'}
										<FlowJobResult
											workspaceId={job?.workspace_id}
											jobId={job?.id}
											filename={job.id}
											loading={job['running']}
											noBorder
											col
											result={job['result']}
											logs={job.logs ?? ''}
										/>
									{:else if selectedNode == 'start'}
										{#if job.args}
											<div class="p-2">
												<JobArgs args={job.args} />
											</div>
										{:else}
											<p class="p-2">No arguments</p>
										{/if}
									{:else if node}
										<div class="px-2 flex gap-2 min-w-0 overflow-hidden w-full">
											<ModuleStatus type={node.type} scheduled_for={node.scheduled_for} />
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
														href="/run/{node.job_id ?? ''}?workspace={job?.workspace_id}"
													>
														{truncateRev(node.job_id ?? '', 10)}
													</a>
												</div>
											{/if}
										</div>
										{#if !node.isListJob}
											<div class="px-1 py-1">
												<JobArgs args={node.args} />
											</div>
										{/if}

										<FlowJobResult
											workspaceId={job?.workspace_id}
											jobId={node.job_id}
											noBorder
											loading={false}
											col
											result={node.result}
											logs={node.logs ?? ''}
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
