<script lang="ts">
	import { writable, type Writable } from 'svelte/store'
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'
	import type { FlowState } from './flows/flowState'
	import { createEventDispatcher, setContext } from 'svelte'
	import type { FlowStatusViewerContext } from './graph'
	import { isOwner as loadIsOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { Job } from '$lib/gen'

	export let jobId: string
	export let initialJob: Job | undefined = undefined
	export let workspaceId: string | undefined = undefined
	export let flowStateStore: Writable<FlowState> = writable({})
	export let selectedJobStep: string | undefined = undefined
	export let hideFlowResult = false
	export let hideTimeline = false
	export let hideDownloadInGraph = false
	export let hideNodeDefinition = false
	export let hideJobId = false
	export let hideDownloadLogs = false
	export let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' =
		'timeline'
	export let isOwner = false
	export let wideResults = false

	let lastJobId: string = jobId

	let retryStatus = writable({})
	let suspendStatus = writable({})
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
			$retryStatus = {}
			$suspendStatus = {}
		}
	}

	const dispatch = createEventDispatcher()

	let lastScriptPath: string | undefined = undefined

	$: jobId && updateJobId()
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
	globalDurationStatuses={[]}
	globalModuleStates={[]}
	bind:selectedNode={selectedJobStep}
	on:start
	on:done
	{initialJob}
	{jobId}
	{workspaceId}
	{isOwner}
	{wideResults}
	bind:rightColumnSelect
/>
