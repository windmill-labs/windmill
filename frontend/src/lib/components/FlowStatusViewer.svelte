<script lang="ts">
	import { writable, type Writable } from 'svelte/store'
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'
	import type { FlowState } from './flows/flowState'
	import { createEventDispatcher, setContext } from 'svelte'
	import type { FlowStatusViewerContext } from './graph'
	import { isOwner as loadIsOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'

	export let jobId: string
	export let workspaceId: string | undefined = undefined
	export let flowStateStore: Writable<FlowState> | undefined = undefined
	export let selectedJobStep: string | undefined = undefined

	export let isOwner = false

	let lastJobId: string = jobId

	let flowModuleStates = writable({})
	let retryStatus = writable({})
	let suspendStatus = writable({})
	let durationStatuses = writable({})
	setContext<FlowStatusViewerContext>('FlowStatusViewer', {
		flowStateStore,
		flowModuleStates,
		retryStatus,
		suspendStatus,
		durationStatuses
	})

	function loadOwner(path: string) {
		isOwner = loadIsOwner(path, $userStore!, workspaceId ?? $workspaceStore!)
	}

	async function updateJobId() {
		if (jobId !== lastJobId) {
			lastJobId = jobId
			$flowModuleStates = {}
			$retryStatus = {}
			$suspendStatus = {}
			$durationStatuses = {}
		}
	}

	const dispatch = createEventDispatcher()

	let lastScriptPath: string | undefined = undefined

	$: jobId && updateJobId()
</script>

<FlowStatusViewerInner
	on:jobsLoaded={({ detail }) => {
		if (detail.script_path != lastScriptPath && detail.script_path) {
			lastScriptPath = detail.script_path
			loadOwner(lastScriptPath ?? '')
		}
		dispatch('jobsLoaded', detail)
	}}
	bind:selectedNode={selectedJobStep}
	{jobId}
	{workspaceId}
	{isOwner}
/>
