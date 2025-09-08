<script lang="ts">
	import type { Job } from '$lib/gen'
	import type { DurationStatus, GraphModuleState } from './graph'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import FlowTimelineCompute from './FlowTimelineCompute.svelte'
	import { untrack } from 'svelte'
	import { ChangeTracker } from '$lib/svelte5Utils.svelte'
	import { readFieldsRecursively } from '$lib/utils'
	import type { NavigationChain } from '$lib/keyboardChain'
	import { getDbClockNow } from '$lib/forLater'

	interface Props {
		job: Partial<Job>
		localModuleStates: Record<string, GraphModuleState>
		localDurationStatuses: Record<string, DurationStatus>
		workspaceId: string | undefined
		render: boolean
		onSelectedIteration: (
			detail:
				| { id: string; index: number; manuallySet: true; moduleId: string }
				| { manuallySet: false; moduleId: string }
		) => Promise<void>
		mode?: 'flow' | 'aiagent'
	}

	let {
		job,
		localModuleStates,
		localDurationStatuses,
		workspaceId,
		render,
		onSelectedIteration,
		mode = 'flow'
	}: Props = $props()

	// State for tracking expanded rows - using Record to allow explicit control
	let expandedRows: Record<string, boolean> = $state({})
	let allExpanded = $state(false)
	let showResultsInputs = $state(true)

	// Keyboard navigation state - incremental like expandedRows
	let currentId = $state<string | null>('flow-root')
	let navigationChain = $state<NavigationChain>({})

	// Timeline state
	let timelineMin = $state<number | undefined>(undefined)
	let timelineMax = $state<number | undefined>(undefined)
	let timelineTotal = $state<number | undefined>(undefined)
	let timelineItems = $state<
		| Record<
				string,
				Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
		  >
		| undefined
	>(undefined)
	let timelineNow = $state<number>(getDbClockNow().getTime())

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
		return localModuleStates[stepId]?.selectedForloopIndex ?? 0
	}

	function toggleExpandAll() {
		allExpanded = !allExpanded
		expandedRows = {}
	}

	// Keyboard event handler using navigation links
	function handleKeydown(event: KeyboardEvent) {
		if (!currentId && job.raw_flow?.modules) {
			currentId = 'flow-root'
		} else if (!currentId) {
			return
		}

		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault()
				const downId = navigationChain[currentId]?.downId
				if (downId) {
					currentId = downId
				}
				break
			case 'ArrowUp':
				event.preventDefault()
				const upId = navigationChain[currentId]?.upId
				if (upId) {
					currentId = upId
				}
				break
			case 'Enter':
				event.preventDefault()
				toggleExpanded(currentId)
				break
		}
	}

	function select(id: string) {
		currentId = id
	}

	const timelineAvailableWidths = $state<Record<string, number>>({})
	const timelinelWidth = $derived(
		Math.max(Math.min(...Object.values(timelineAvailableWidths)) - 12, 0)
	)
</script>

<div
	class="w-full rounded-md overflow-hidden border focus:border-gray-400 dark:focus:border-gray-400"
	role="tree"
	tabindex="0"
	onkeydown={handleKeydown}
>
	{#if localDurationStatuses}
		<FlowTimelineCompute
			flowModules={modules.map((m) => m.id)}
			durationStatuses={localDurationStatuses}
			flowDone={job.type === 'CompletedJob'}
			bind:min={timelineMin}
			bind:max={timelineMax}
			bind:total={timelineTotal}
			bind:items={timelineItems}
			bind:now={timelineNow}
		/>
	{/if}
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
		{mode}
		{currentId}
		bind:navigationChain
		{select}
		{timelineMin}
		{timelineTotal}
		{timelineItems}
		{timelineNow}
		{timelineAvailableWidths}
		{timelinelWidth}
	/>
</div>
