<script lang="ts">
	import Button from './common/button/Button.svelte'
	import { ChevronDown } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'
	import { JobService, type Job, type FlowModuleValue } from '$lib/gen'
	import type { GraphModuleState, DurationStatus } from './graph'
	import type { Writable } from 'svelte/store'

	interface Props {
		flowJobIds:
			| {
					moduleId: string
					flowJobs: string[]
					flowJobsSuccess: (boolean | undefined)[]
					length: number
					branchall?: boolean
			  }
			| undefined
		render: boolean
		forloop_selected: string | undefined
		sliceFrom: number
		storedListJobs: Record<number, Job>
		workspaceId: string | undefined
		globalRefreshes: Record<string, (clear, root) => Promise<void>>
		recursiveRefresh: Record<string, (clear, root) => Promise<void>>
		childFlow: boolean
		localModuleStates: Writable<Record<string, GraphModuleState>>
		globalModuleStates: Writable<Record<string, GraphModuleState>>[]
		localDurationStatuses: Writable<Record<string, DurationStatus>>
		globalDurationStatuses: Writable<Record<string, DurationStatus>>[]
		prefix: string | undefined
		subflowParentsGlobalModuleStates: Writable<Record<string, GraphModuleState>>[]
		subflowParentsDurationStatuses: Writable<Record<string, DurationStatus>>[]
		selected: string
		innerModule: FlowModuleValue | undefined
		reducedPolling: boolean
		innerJobLoaded: (job: Job, index: number, clicked: boolean, force: boolean) => void
	}

	let {
		flowJobIds,
		render,
		forloop_selected,
		sliceFrom,
		storedListJobs = $bindable({}),
		workspaceId,
		globalRefreshes,
		recursiveRefresh,
		childFlow,
		localModuleStates,
		globalModuleStates,
		localDurationStatuses,
		globalDurationStatuses,
		prefix,
		subflowParentsGlobalModuleStates,
		subflowParentsDurationStatuses,
		selected,
		innerModule,
		reducedPolling,
		innerJobLoaded
	}: Props = $props()

	let subflowsSize = $state(500)
</script>

<h3 class="text-md leading-6 font-bold text-tertiary border-b mb-4">
	Subflows ({flowJobIds?.flowJobs.length})
</h3>
<div class="overflow-auto max-h-1/2">
	{#each flowJobIds?.flowJobs ?? [] as loopJobId, j (loopJobId)}
		{#if render && j + subflowsSize + 1 == (flowJobIds?.flowJobs.length ?? 0)}
			<Button variant="border" color="light" on:click={() => (subflowsSize += 500)}
				>Load 500 more...</Button
			>
		{/if}
		{#if render && (j + subflowsSize + 1 > (flowJobIds?.flowJobs.length ?? 0) || forloop_selected == loopJobId)}
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
							noLogs: true,
							noCode: true
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
			{@const forloopIsSelected =
				forloop_selected == loopJobId ||
				(innerModule?.type != 'forloopflow' && innerModule?.type != 'whileloopflow')}
			<!-- <LogId id={loopJobId} /> -->
			<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
				<FlowStatusViewerInner
					{globalRefreshes}
					parentRecursiveRefresh={recursiveRefresh}
					{childFlow}
					job={storedListJobs[j]}
					initialJob={storedListJobs[j]}
					globalModuleStates={forloopIsSelected ? [localModuleStates, ...globalModuleStates] : []}
					globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
					{prefix}
					subflowParentsGlobalModuleStates={forloopIsSelected
						? subflowParentsGlobalModuleStates
						: []}
					{subflowParentsDurationStatuses}
					render={forloop_selected == loopJobId && selected == 'sequence' && render}
					isForloopSelected={forloop_selected == loopJobId &&
						(innerModule?.type == 'forloopflow' || innerModule?.type == 'whileloopflow')}
					reducedPolling={reducedPolling ||
						(!!flowJobIds?.flowJobs.length && flowJobIds?.flowJobs.length > 20)}
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
