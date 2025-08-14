<script lang="ts">
	import type { Job } from '$lib/gen'
	import { type Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import { untrack } from 'svelte'
	import { ChangeTracker } from '$lib/svelte5Utils.svelte'
	import { readFieldsRecursively } from '$lib/utils'

	interface Props {
		job: Job
		localModuleStates: Writable<Record<string, GraphModuleState>>
		workspaceId: string | undefined
		render: boolean
		onSelectedIteration: (
			detail:
				| { id: string; index: number; manuallySet: true; moduleId: string }
				| { manuallySet: false; moduleId: string }
		) => Promise<void>
	}

	let { job, localModuleStates, workspaceId, render, onSelectedIteration }: Props = $props()

	// State for tracking expanded rows - using Record to allow explicit control
	let expandedRows: Record<string, boolean> = $state({})
	let allExpanded = $state(false)
	let showResultsInputs = $state(true)

	let moduleTracker = new ChangeTracker($state.snapshot(job.raw_flow?.modules ?? []))
	$effect(() => {
		readFieldsRecursively(job.raw_flow?.modules ?? [])
		untrack(() => moduleTracker.track($state.snapshot(job.raw_flow?.modules ?? [])))
	})
	let modules = $derived.by(() => {
		moduleTracker.counter
		return untrack(() => job.raw_flow?.modules ?? [])
	})

	function toggleExpanded(id: string) {
		// If not in record, use opposite of allExpanded as new state
		// If in record, toggle the current state
		const currentState = expandedRows[id] ?? allExpanded
		expandedRows[id] = !currentState
	}

	function getSelectedIteration(stepId: string): number {
		return $localModuleStates[stepId]?.selectedForloopIndex ?? 0
	}

	function toggleExpandAll() {
		allExpanded = !allExpanded
		expandedRows = {}
	}
</script>

<div class="w-full rounded-md overflow-hidden border">
	<FlowLogViewer
		{modules}
		{localModuleStates}
		rootJob={job}
		{expandedRows}
		{allExpanded}
		{showResultsInputs}
		{toggleExpanded}
		{toggleExpandAll}
		{onSelectedIteration}
		{workspaceId}
		{render}
		{getSelectedIteration}
		flowId="root"
		flowStatus={undefined}
	/>
</div>
