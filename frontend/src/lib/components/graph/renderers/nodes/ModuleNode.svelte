<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import type { FlowModule, FlowModuleValue } from '$lib/gen'
	import { GitBranchPlus } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { GraphModuleState } from '../../model'
	import { getStateColor } from '../../util'

	export let data: {
		offset: number
		value: FlowModuleValue
		module: FlowModule
		insertable: boolean
		insertableEnd: boolean
		branchable: boolean
		bgColor: string
		modules: FlowModule[]
		moving: string | undefined
		disableAi: boolean
		wrapperId: string | undefined
		retries: number | undefined
		flowJobs:
			| { flowJobs: string[]; selected: number; flowJobsSuccess: (boolean | undefined)[] }
			| undefined
		eventHandlers: GraphEventHandlers
		flowModuleStates: Record<string, GraphModuleState> | undefined
		selected: boolean
	}

	$: type = data.flowModuleStates?.[data.module.id]?.type
	if (!type && data.flowJobs) {
		type = 'InProgress'
	}

	$: state = data.flowModuleStates?.[data.module.id]
	$: flowJobs = state?.flow_jobs
		? {
				flowJobs: state?.flow_jobs,
				selected: state?.selectedForloopIndex ?? -1,
				flowJobsSuccess: state?.flow_jobs_success
		  }
		: (undefined as any)

	let selectedIteration = -1
</script>

<NodeWrapper offset={data.offset} let:darkMode>
	<MapItem
		mod={data.module}
		insertable={data.insertable}
		annotation={flowJobs &&
		(data.module.value.type === 'forloopflow' || data.module.value.type === 'whileloopflow')
			? 'Iteration: ' +
			  (selectedIteration >= 0 ? selectedIteration : state?.flow_jobs?.length) +
			  '/' +
			  (state?.iteration_total ?? '?')
			: ''}
		bgColor={getStateColor(type, darkMode, true, state?.skipped)}
		modules={data.modules ?? []}
		moving={data.moving}
		duration_ms={state?.duration_ms}
		retries={data.retries}
		disableAi={data.disableAi}
		{flowJobs}
		on:delete={(e) => {
			data.eventHandlers.delete(e.detail, '')
		}}
		on:insert={(e) => {
			data.eventHandlers.insert(e.detail)
		}}
		on:changeId={(e) => {
			data.eventHandlers.changeId(e.detail)
		}}
		on:move={(e) => {
			data.eventHandlers.move(data.module, data.modules)
		}}
		on:newBranch={(e) => {
			data.eventHandlers.newBranch(data.module)
		}}
		on:select={(e) => {
			data.eventHandlers.select(e.detail)
		}}
		on:selectedIteration={(e) => {
			selectedIteration = e.detail.index + 1
			data.eventHandlers.selectedIteration(e.detail, data.module.id)
		}}
	/>

	<div class="absolute -bottom-10 left-1/2 transform -translate-x-1/2 z-10">
		{#if (data.value.type === 'branchall' || data.value.type === 'branchone') && data.insertable}
			<button
				title="Add branch"
				class="rounded text-secondary border hover:bg-surface-hover bg-surface p-1"
				on:click={() => {
					data?.eventHandlers?.newBranch(data.module)
				}}
			>
				<GitBranchPlus size={16} />
			</button>
		{/if}
	</div>
</NodeWrapper>
