<script lang="ts">
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'
	import type { FlowState } from './flows/flowState'
	import { setContext, untrack } from 'svelte'
	import type { DurationStatus, FlowStatusViewerContext, GraphModuleState } from './graph'
	import { isOwner as loadIsOwner, type StateStore } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { CompletedJob, Job } from '$lib/gen'

	interface Props {
		jobId: string
		initialJob?: Job | undefined
		workspaceId?: string | undefined
		flowState?: FlowState
		selectedJobStep?: string | undefined
		hideFlowResult?: boolean
		hideTimeline?: boolean
		hideDownloadInGraph?: boolean
		hideNodeDefinition?: boolean
		hideJobId?: boolean
		hideDownloadLogs?: boolean
		rightColumnSelect?: 'timeline' | 'node_status' | 'node_definition' | 'user_states'
		isOwner?: boolean
		wideResults?: boolean
		localModuleStates?: Record<string, GraphModuleState>
		localDurationStatuses?: Record<string, DurationStatus>
		job?: Job | undefined
		render?: boolean
		suspendStatus?: StateStore<Record<string, { job: Job; nb: number }>>
		customUi?: {
			tagLabel?: string | undefined
		}
		onStart?: () => void
		onJobsLoaded?: ({ job, force }: { job: Job; force: boolean }) => void
		onDone?: ({ job }: { job: CompletedJob }) => void
	}

	let {
		jobId,
		initialJob = undefined,
		workspaceId = undefined,
		flowState = $bindable({}),
		selectedJobStep = $bindable(undefined),
		hideFlowResult = false,
		hideTimeline = false,
		hideDownloadInGraph = false,
		hideNodeDefinition = false,
		hideJobId = false,
		hideDownloadLogs = false,
		rightColumnSelect = $bindable('timeline'),
		isOwner = $bindable(false),
		wideResults = false,
		localModuleStates = $bindable({}),
		localDurationStatuses = $bindable({}),
		job = $bindable(undefined),
		render = true,
		suspendStatus = $bindable({ val: {} }),
		customUi,
		onStart,
		onJobsLoaded,
		onDone
	}: Props = $props()

	let lastJobId: string = jobId

	let retryStatus = $state({ val: {} })
	let globalRefreshes: Record<string, ((clear, root) => Promise<void>)[]> = $state({})

	setContext<FlowStatusViewerContext>('FlowStatusViewer', {
		flowState,
		suspendStatus,
		retryStatus,
		hideDownloadInGraph,
		hideNodeDefinition,
		hideTimeline,
		hideJobId,
		hideDownloadLogs
	})

	function loadOwner(path: string) {
		isOwner = loadIsOwner(path, $userStore!, workspaceId ?? $workspaceStore!)
	}

	async function updateJobId() {
		if (jobId !== lastJobId) {
			console.log('updateJobId 3', jobId)
			lastJobId = jobId
			retryStatus.val = {}
			suspendStatus.val = {}
			globalRefreshes = {}
			for (let key in localModuleStates) delete flowState[key]
			localDurationStatuses = {}
			localModuleStates = {}
		}
	}

	let lastScriptPath: string | undefined = $state(undefined)

	$effect.pre(() => {
		jobId
		untrack(() => {
			jobId && updateJobId()
		})
	})

	let refreshGlobal = async (moduleId: string, clear: boolean, root: string) => {
		let allFns = globalRefreshes?.[moduleId]?.map((x) => x(clear, root)) ?? []
		await Promise.all(allFns)
	}

	let updateGlobalRefresh = (moduleId: string, updateFn: (clear, root) => Promise<void>) => {
		globalRefreshes[moduleId] = [...(globalRefreshes[moduleId] ?? []), updateFn]
	}

	let storedToolCallJobs: Record<string, Job> = $state({})
	let toolCallIndicesToLoad: string[] = $state([])
</script>

{#key jobId}
	<FlowStatusViewerInner
		{hideFlowResult}
		onJobsLoaded={({ job, force }) => {
			if (job.script_path != lastScriptPath && job.script_path) {
				lastScriptPath = job.script_path
				loadOwner(lastScriptPath ?? '')
			}
			onJobsLoaded?.({ job, force })
		}}
		globalModuleStates={[]}
		bind:localModuleStates
		bind:selectedNode={selectedJobStep}
		bind:localDurationStatuses
		{onStart}
		{onDone}
		bind:job
		{initialJob}
		{jobId}
		{workspaceId}
		{isOwner}
		{wideResults}
		bind:rightColumnSelect
		{render}
		{customUi}
		graphTabOpen={true}
		isNodeSelected={true}
		{refreshGlobal}
		{updateGlobalRefresh}
		toolCallStore={{
			getStoredToolCallJob: (storeKey: string) => storedToolCallJobs[storeKey],
			setStoredToolCallJob: (storeKey: string, job: Job) => {
				storedToolCallJobs[storeKey] = job
			},
			getLocalToolCallJobs: (prefix: string) => {
				// we return a map from tool call index to job
				// to do so, we filter the storedToolCallJobs object by the prefix and we make sure what's left in the key is a tool call index: 2 part of format agentModuleId-toolCallIndex
				// and not a further nested tool call index
				return Object.fromEntries(
					Object.entries(storedToolCallJobs)
						.filter(
							([key]) => key.startsWith(prefix) && key.replace(prefix, '').split('-').length === 2
						)
						.map(([key, job]) => [Number(key.replace(prefix, '').split('-').pop()), job])
				)
			},
			isToolCallToBeLoaded: (storeKey: string) => {
				return toolCallIndicesToLoad.includes(storeKey)
			},
			addToolCallToLoad: (storeKey: string) => {
				if (!toolCallIndicesToLoad.includes(storeKey)) {
					toolCallIndicesToLoad.push(storeKey)
				}
			}
		}}
	/>
{/key}
