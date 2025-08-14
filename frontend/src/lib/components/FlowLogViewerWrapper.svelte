<script lang="ts">
	import type { Job } from '$lib/gen'
	import { writable, type Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import { graphBuilder, type NodeLayout } from './graph/graphBuilder.svelte'
	import type { FlowLogEntry } from './FlowLogUtils'
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

	let emptyEventHandler = {
		deleteBranch: () => {},
		insert: () => {},
		select: () => {},
		changeId: () => {},
		delete: () => {},
		newBranch: () => {},
		move: () => {},
		selectedIteration: () => {},
		simplifyFlow: () => {},
		expandSubflow: () => {},
		minimizeSubflow: () => {},
		updateMock: () => {},
		testUpTo: () => {},
		editInput: () => {},
		testFlow: () => {},
		cancelTestFlow: () => {},
		openPreview: () => {},
		hideJobStatus: () => {}
	}

	let modules = $derived(job.raw_flow?.modules ?? [])
	let moduleTracker = new ChangeTracker($state.snapshot(job.raw_flow?.modules ?? []))
	$effect(() => {
		readFieldsRecursively(modules)
		untrack(() => moduleTracker.track($state.snapshot(modules)))
	})

	let nodes: NodeLayout[] | undefined = $derived.by(() => {
		moduleTracker.counter
		const graph = graphBuilder(
			untrack(() => modules),
			{
				disableAi: false,
				insertable: false,
				flowModuleStates: undefined,
				selectedId: undefined,
				path: undefined,
				newFlow: false,
				cache: false,
				earlyStop: false,
				editMode: false,
				isOwner: false,
				isRunning: false,
				individualStepTests: false,
				flowJob: undefined,
				showJobStatus: false,
				suspendStatus: writable({}),
				flowHasChanged: false
			},
			untrack(() => job.raw_flow?.failure_module),
			untrack(() => job.raw_flow?.preprocessor_module),
			emptyEventHandler, // eventHandler - empty for logs view
			undefined,
			false, // useDataflow
			undefined, // selectedId
			undefined, // moving
			undefined, // simplifiableFlow
			undefined, // triggerNode path
			{} // expandedSubflows
		)
		return graph.error ? undefined : graph.nodes
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

	// Build tree structure from modules using bottom-up traversal from result node
	function buildFlowTree(nodes: NodeLayout[]): FlowLogEntry[] {
		// Index nodes for quick access
		const nodeById: Record<string, NodeLayout> = {}
		for (const n of nodes) nodeById[n.id] = n

		function traverseFromId(id: string): FlowLogEntry[] {
			const entries: FlowLogEntry[] = []
			const currentNode = nodeById[id]

			if (currentNode.type === 'module') {
				entries.push({
					id: currentNode.id,
					stepId: currentNode.id,
					stepNumber: 0,
					summary: currentNode.data.module.summary ?? '',
					stepType: currentNode.data.module.value.type
				})
			}

			if (
				currentNode.type === 'whileLoopStart' ||
				currentNode.type === 'branchOneStart' ||
				currentNode.type === 'branchAllStart' ||
				currentNode.type === 'forLoopStart'
			) {
				// Reaching the end of a subflow
				return entries
			}

			let nextParentId = currentNode.parentIds?.[0]
			if (!nextParentId) {
				// Reached the root of the flow
				return entries
			}

			if (currentNode.type === 'forLoopEnd' || currentNode.type === 'whileLoopEnd') {
				const subflow = traverseFromId(nextParentId)
				const subflowId = currentNode.id.slice(0, -4) // Remove '-end' suffix
				const subflowNode = nodeById[subflowId]
				const subflowSummary = 'iteration'
				if (subflowNode.type === 'module') {
					entries.push({
						id: subflowId,
						stepId: subflowId,
						subflows: [subflow],
						stepType: subflowNode.data.module.value.type,
						summary: subflowNode.data.module.summary ?? '',
						subflowsSummary: [subflowSummary]
					})
					nextParentId = subflowNode.parentIds?.[0] ?? ''
				}
			} else if (currentNode.type === 'branchAllEnd' || currentNode.type === 'branchOneEnd') {
				const subflowId = currentNode.id.slice(0, -4) // Remove '-end' suffix
				const subflowNode = nodeById[subflowId]
				if (
					subflowNode.type === 'module' &&
					(subflowNode.data.module.value.type === 'branchall' ||
						subflowNode.data.module.value.type === 'branchone')
				) {
					const subflows = currentNode.parentIds?.map((id) => traverseFromId(id)) ?? []
					const subflowsSummary = subflowNode.data.module.value.branches.map((b) => b.summary ?? '')
					entries.push({
						id: subflowId,
						stepId: subflowId,
						subflows: subflows,
						stepType: subflowNode.data.module.value.type,
						subflowsSummary
					})
					nextParentId = subflowNode.parentIds?.[0] ?? '' // a module can only have one parent
				}
			}

			// Get entries from parent nodes
			const parentEntries = traverseFromId(nextParentId)
			return [...parentEntries, ...entries]
		}

		// Start from the result node and traverse backwards
		return traverseFromId('result')
	}

	let logEntries = $derived(nodes ? buildFlowTree(nodes) : [])
</script>

<div class="w-full rounded-md overflow-hidden border">
	<FlowLogViewer
		{logEntries}
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
