<script lang="ts">
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'
	import type { FlowState } from './flows/flowState'
	import { setContext, untrack } from 'svelte'
	import type { DurationStatus, FlowStatusViewerContext, GraphModuleState } from './graph'
	import { isOwner as loadIsOwner, type StateStore } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { CompletedJob, FlowModule, FlowNote, FlowValue, Job } from '$lib/gen'

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
		/** Cached subflow definitions, keyed by graph node id (subflow step id, with
		 * `subflow:` prefix for nested subflows). Populated as the user expands a
		 * subflow in the graph. Bindable so callers can use it to walk inside
		 * subflows (e.g. for nested-restart path-finding). */
		expandedSubflows?: Record<string, { modules: FlowModule[]; groups?: any[] }>
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
		showLogsWithResult?: boolean
		notes?: FlowNote[]
		groups?: FlowValue['groups']
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
		// `$bindable({})` is the right shape for this prop despite CLAUDE.md's
		// general guidance against `$bindable(default_value)`: that rule targets
		// props where `undefined` has distinct semantics from "empty default".
		// For a Map-typed cache populated by the inner component, callers that
		// don't bind it should still see a usable empty map (the inner writes to
		// it when the user expands a subflow). Without the `{}` default, those
		// callers would crash at write time. Same reasoning applies to the
		// pre-existing `localModuleStates`/`localDurationStatuses` bindables.
		expandedSubflows = $bindable({}),
		localDurationStatuses = $bindable({}),
		job = $bindable(undefined),
		render = true,
		suspendStatus = $bindable({ val: {} }),
		customUi,
		onStart,
		onJobsLoaded,
		onDone,
		showLogsWithResult = false,
		notes: notesProp = undefined,
		groups: groupsProp = undefined
	}: Props = $props()

	let lastJobId: string = untrack(() => jobId)

	let retryStatus = $state({ val: {} })
	let globalRefreshes: Record<string, ((clear, root) => Promise<void>)[]> = $state({})

	setContext<FlowStatusViewerContext>('FlowStatusViewer', {
		flowState,
		suspendStatus,
		retryStatus,
		hideDownloadInGraph: untrack(() => hideDownloadInGraph),
		hideNodeDefinition: untrack(() => hideNodeDefinition),
		hideTimeline: untrack(() => hideTimeline),
		hideJobId: untrack(() => hideJobId),
		hideDownloadLogs: untrack(() => hideDownloadLogs)
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
			// Reset the subflow definition cache too — a new run can reference
			// different subflow versions; stale entries would confuse nested
			// restart path-finding.
			expandedSubflows = {}
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
		onJobsLoaded={({ job, force }) => {
			if (job.script_path != lastScriptPath && job.script_path) {
				lastScriptPath = job.script_path
				loadOwner(lastScriptPath ?? '')
			}
			onJobsLoaded?.({ job, force })
		}}
		globalModuleStates={[]}
		bind:localModuleStates
		bind:expandedSubflows
		bind:selectedNode={selectedJobStep}
		bind:localDurationStatuses
		{onStart}
		{onDone}
		bind:job
		{initialJob}
		{jobId}
		{workspaceId}
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
		{showLogsWithResult}
		{hideFlowResult}
		notes={notesProp}
		groups={groupsProp}
	/>
{/key}
