<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import FlowJobsMenu from '$lib/components/flows/map/FlowJobsMenu.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Minimize2 } from 'lucide-svelte'
	import type { SubflowBoundN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'

	interface Props {
		data: SubflowBoundN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()
	const flowRunStatus = getFlowRunStatusContext()

	// An expanded subflow step is drawn as these bounds, so its start carries the step's attempt
	// picker, as a loop's node keeps its iteration picker while its body shows.
	let attempts = $derived.by(() => {
		if (id !== data.subflowId) return undefined
		const state = flowRunStatus?.getModuleState(data.subflowId)
		const jobs = state?.attempt_jobs
		if (!jobs?.length) return undefined
		const shown = state?.selectedAttempt ? jobs.indexOf(state.selectedAttempt) : -1
		return {
			jobs,
			success: state?.attempt_jobs_success ?? [],
			selected: shown >= 0 ? shown : jobs.length - 1
		}
	})
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		{#if attempts}
			<div class="absolute z-10 left-0 -top-5 center-center text-primary">
				Attempt: {attempts.selected + 1}/{attempts.jobs.length}
			</div>
			<div class="absolute right-20 z-50 -top-5">
				<FlowJobsMenu
					moduleId={data.subflowId}
					id={data.subflowId}
					onSelectedIteration={(detail) => data.eventHandlers.selectedIteration(detail)}
					flowJobs={attempts.jobs}
					flowJobsSuccess={attempts.success}
					selected={attempts.selected}
					selectedManually={false}
					label="attempt"
				/>
			</div>
		{/if}
		<VirtualItem
			label={data.label}
			preLabel={data.preLabel}
			selectable
			selected={selectionManager && selectionManager.isNodeSelected(id)}
			on:select={() => {
				setTimeout(() => data.eventHandlers?.select(data.id))
			}}
		/>
		<div class="z-50 absolute -top-4 right-11 rounded-md text-primary bg-surface">
			<button
				title="Unexpand subflow"
				class="rounded-md center-center text-primary hover:bg-surface-tertiary shadow-md p-1 duration-0"
				onclick={stopPropagation(
					preventDefault(() => {
						data.eventHandlers.minimizeSubflow(data.subflowId)
					})
				)}
			>
				<Minimize2 size={12} />
			</button>
		</div>
	{/snippet}
</NodeWrapper>
