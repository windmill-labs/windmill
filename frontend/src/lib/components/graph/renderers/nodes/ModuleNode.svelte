<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import type { FlowModule, FlowModuleValue } from '$lib/gen'
	import { GitBranchPlus, Maximize2 } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { GraphModuleState } from '../../model'
	import { getStateColor, getStateHoverColor } from '../../util'

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
		editMode: boolean
	}

	$: type = data.flowModuleStates?.[data.module.id]?.type
	if (!type && data.flowJobs) {
		type = 'InProgress'
	}

	$: state = data.flowModuleStates?.[data.module.id]
	$: flowJobs = state?.flow_jobs
		? {
				flowJobs: state?.flow_jobs,
				selected: state?.selectedForloopIndex ?? 0,
				selectedManually: state?.selectedForLoopSetManually,
				flowJobsSuccess: state?.flow_jobs_success
			}
		: (undefined as any)
</script>

<NodeWrapper offset={data.offset} let:darkMode>
	{#if data.module.value.type == 'flow'}
		<button
			title="Expand subflow"
			class="z-50 absolute -top-[10px] right-[25px] rounded-full h-[20px] w-[20px] center-center text-primary bg-surface duration-0 hover:bg-surface-hover"
			on:click|preventDefault|stopPropagation={() => {
				if (data.module.value.type == 'flow') {
					data.eventHandlers.expandSubflow(data.module.id, data.module.value.path)
				}
			}}
		>
			<Maximize2 size={12} />
		</button>
	{/if}
	<MapItem
		mod={data.module}
		insertable={data.insertable}
		editMode={data.editMode}
		annotation={flowJobs &&
		(data.module.value.type === 'forloopflow' || data.module.value.type === 'whileloopflow')
			? 'Iteration: ' +
				((state?.selectedForloopIndex ?? 0) >= 0
					? (state?.selectedForloopIndex ?? 0) + 1
					: state?.flow_jobs?.length) +
				'/' +
				(state?.iteration_total ?? '?')
			: ''}
		bgColor={getStateColor(type, darkMode, true, state?.skipped)}
		bgHoverColor={getStateHoverColor(type, darkMode, true, state?.skipped)}
		moving={data.moving}
		duration_ms={state?.duration_ms}
		retries={data.retries}
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
			setTimeout(() => data.eventHandlers.select(e.detail))
		}}
		on:selectedIteration={(e) => {
			data.eventHandlers.selectedIteration(e.detail, data.module.id)
		}}
		on:updateMock={() => {
			data.eventHandlers.updateMock()
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
