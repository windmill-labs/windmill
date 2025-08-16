<script lang="ts">
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'
	import type { FlowState } from './flows/flowState'
	import { createEventDispatcher, setContext, untrack } from 'svelte'
	import type { DurationStatus, FlowStatusViewerContext, GraphModuleState } from './graph'
	import { isOwner as loadIsOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { Job } from '$lib/gen'

	interface Props {
		jobId: string
		initialJob?: Job | undefined
		workspaceId?: string | undefined
		flowStateStore?: FlowState
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
		suspendStatus?: any
		customUi?: {
			tagLabel?: string | undefined
		}
	}

	let {
		jobId,
		initialJob = undefined,
		workspaceId = undefined,
		flowStateStore = $bindable({}),
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
		suspendStatus = $bindable({}),
		customUi
	}: Props = $props()

	let lastJobId: string = jobId

	let retryStatus = $state({})
	setContext<FlowStatusViewerContext>('FlowStatusViewer', {
		flowStateStore,
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
			lastJobId = jobId
			retryStatus = {}
			suspendStatus = {}
		}
	}

	const dispatch = createEventDispatcher()

	let lastScriptPath: string | undefined = $state(undefined)

	$effect.pre(() => {
		jobId
		untrack(() => {
			jobId && updateJobId()
		})
	})
</script>

<FlowStatusViewerInner
	{hideFlowResult}
	on:jobsLoaded={({ detail }) => {
		let { job } = detail
		if (job.script_path != lastScriptPath && job.script_path) {
			lastScriptPath = job.script_path
			loadOwner(lastScriptPath ?? '')
		}
		dispatch('jobsLoaded', job)
	}}
	globalModuleStates={[]}
	{localModuleStates}
	bind:selectedNode={selectedJobStep}
	on:start
	on:done
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
/>
