<script lang="ts">
	import type { Job } from '$lib/gen'
	import type { Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import FlowLogsLoader from './FlowLogsLoader.svelte'
	import FlowStepsLogsLoader from './FlowStepsLogsLoader.svelte'
	import type { NodeLayout } from './graph/graphBuilder.svelte'
	import type { FlowLogEntry } from './FlowLogUtils'

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
		nodes: NodeLayout[] | undefined
	}

	let { job, localModuleStates, workspaceId, render, onSelectedIteration, nodes }: Props = $props()

	// State for tracking expanded rows - using Record to allow explicit control
	let expandedRows: Record<string, boolean> = $state({})
	let allExpanded = $state(false)
	let showResultsInputs = $state(true)

	// Root flow logs
	let rootFlowLogs: string | undefined = $state(undefined)

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
				currentNode.type === 'input2' ||
				currentNode.type === 'forLoopStart' ||
				currentNode.type === 'whileLoopStart' ||
				currentNode.type === 'branchOneStart' ||
				currentNode.type === 'branchAllStart'
			) {
				// Reaching the end of a subflow
				return entries
			}

			let nextParentId = currentNode.parentIds?.[0]
			if (!nextParentId) {
				return entries
			}

			if (
				currentNode.type === 'forLoopEnd' ||
				currentNode.type === 'whileLoopEnd' ||
				currentNode.type === 'branchOneEnd'
			) {
				const subflow = traverseFromId(nextParentId)
				const subflowId = currentNode.id.slice(0, -4) // Remove '-end' suffix
				const subflowNode = nodeById[subflowId]
				const subflowSummary = currentNode.type === 'branchOneEnd' ? 'branch' : 'iteration'
				if (subflowNode.type === 'module') {
					entries.push({
						id: subflowId,
						stepId: subflowId,
						subflows: [subflow],
						stepType:
							currentNode.type === 'forLoopEnd'
								? 'forloopflow'
								: currentNode.type === 'whileLoopEnd'
									? 'whileloopflow'
									: 'branchone',
						summary: subflowNode.data.module.summary ?? '',
						subflowsSummary: [subflowSummary]
					})
					nextParentId = subflowNode.parentIds?.[0] ?? ''
				}
			} else if (currentNode.type === 'branchAllEnd') {
				const subflowId = currentNode.id.slice(0, -4) // Remove '-end' suffix
				const subflowNode = nodeById[subflowId]
				if (subflowNode.type === 'module' && subflowNode.data.module.value.type === 'branchall') {
					const subflows = currentNode.parentIds?.map((id) => traverseFromId(id)) ?? []
					const subflowsSummary = subflowNode.data.module.value.branches.map((b) => b.summary ?? '')
					entries.push({
						id: subflowId,
						stepId: subflowId,
						subflows: subflows,
						stepType: 'branchall',
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
	<!-- Log polling component for the expanded rows -->
	<FlowStepsLogsLoader {expandedRows} {allExpanded} {workspaceId} {localModuleStates} />

	<!-- Log polling component for the root job -->
	<FlowLogsLoader
		loading={job['running'] == true}
		jobId={job.id}
		{workspaceId}
		refreshLog={job.type === 'QueuedJob'}
		bind:logs={rootFlowLogs}
	/>

	<FlowLogViewer
		{logEntries}
		{localModuleStates}
		rootJob={job}
		rootLogs={rootFlowLogs}
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
	/>
</div>
