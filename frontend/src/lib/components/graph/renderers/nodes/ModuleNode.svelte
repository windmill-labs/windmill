<script lang="ts">
	import { NodeToolbar, Position } from '@xyflow/svelte'
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import type { FlowModule, FlowModuleValue } from '$lib/gen/types.gen'
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
		duration_ms: number | undefined
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

	const state = data.flowModuleStates?.[data.module.id]
	const flowJobs = state?.flow_jobs
		? {
				flowJobs: state?.flow_jobs,
				selected: state?.selectedForloopIndex ?? -1,
				flowJobsSuccess: state?.flow_jobs_success
		  }
		: (undefined as any)
</script>

<NodeWrapper offset={data.offset} let:darkMode>
	<MapItem
		mod={data.module}
		insertable={data.insertable}
		annotation={state?.flow_jobs
			? 'ierations ' + state?.flow_jobs?.length + '/' + (state?.iteration_total ?? '?')
			: ''}
		bgColor={getStateColor(type, darkMode)}
		modules={data.modules ?? []}
		moving={data.moving}
		duration_ms={data.duration_ms}
		retries={data.retries}
		{flowJobs}
		on:delete={(e) => {
			data.eventHandlers.delete(e.detail, '')
		}}
		on:insert={(e) => {
			data.eventHandlers.insert(e.detail)
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
			data.eventHandlers.selectIteration(e.detail, data.module.id)
		}}
	/>
</NodeWrapper>

<NodeToolbar isVisible position={Position.Bottom} align="center">
	{#if data.value.type === 'branchall' && data.insertable}
		<button
			class="rounded-full border hover:bg-surface-hover bg-surface p-1"
			on:click={() => {
				data?.eventHandlers?.newBranch(data.module)
			}}
		>
			<GitBranchPlus size={16} />
		</button>
	{/if}
</NodeToolbar>
