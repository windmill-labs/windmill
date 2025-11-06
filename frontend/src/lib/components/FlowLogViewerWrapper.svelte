<script lang="ts">
	import type { Job } from '$lib/gen'
	import type { DurationStatus, GraphModuleState } from './graph'
	import FlowLogViewer from './FlowLogViewer.svelte'
	import { TimelineCompute } from '$lib/timelineCompute.svelte'
	import { onMount, untrack } from 'svelte'
	import { ChangeTracker } from '$lib/svelte5Utils.svelte'
	import { readFieldsRecursively } from '$lib/utils'
	import type { NavigationChain } from '$lib/keyboardChain'
	import OnChange from './common/OnChange.svelte'

	interface Props {
		job: Partial<Job>
		localDurationStatuses?: Record<string, DurationStatus>
		workspaceId: string | undefined
		render: boolean
		localModuleStates: Record<string, GraphModuleState>
		onSelectedIteration?: (
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
	let timelineCompute = $state<TimelineCompute | undefined>(undefined)

	onMount(() => {
		timelineCompute = new TimelineCompute(
			modules.map((m) => m.id),
			localDurationStatuses ?? {},
			job.type === 'CompletedJob'
		)
		return () => {
			timelineCompute?.destroy()
		}
	})

	// Derived timeline values
	const timelineMin = $derived(timelineCompute?.min ?? undefined)
	const timelineTotal = $derived(timelineCompute?.total ?? undefined)
	const timelineItems = $derived(timelineCompute?.items ?? undefined)
	const timelineNow = $derived(timelineCompute?.now ?? Date.now())

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

	let timelineAvailableWidths = $state<Record<string, number>>({})
	let lastJobId: string | undefined = $state(job.id)

	const timelinelWidth = $derived.by(() => {
		const widths = Object.values(timelineAvailableWidths)
		return widths.length > 0 ? Math.max(Math.min(...widths) - 12, 0) : 0
	})

	function updateJobId() {
		if (job.id !== lastJobId) {
			lastJobId = job.id
			navigationChain = {}
			timelineAvailableWidths = {}
			currentId = 'flow-root'
			showResultsInputs = true
			timelineCompute?.reset()
		}
	}

	$effect.pre(() => {
		job.id
		untrack(() => {
			job.id && updateJobId()
		})
	})
</script>

<OnChange
	key={localDurationStatuses}
	onChange={() => {
		timelineCompute?.updateInputs(
			modules.map((m) => m.id),
			localDurationStatuses ?? {},
			job.type === 'CompletedJob'
		)
	}}
/>

<div
	class="w-full rounded-md overflow-hidden border focus:border-gray-400 dark:focus:border-gray-400"
	role="tree"
	tabindex="0"
	onkeydown={handleKeydown}
>
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
		{mode}
		{currentId}
		bind:navigationChain
		{select}
		{timelineMin}
		{timelineTotal}
		{timelineItems}
		{timelineNow}
		bind:timelineAvailableWidths
		{timelinelWidth}
	/>
</div>
