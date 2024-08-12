<script lang="ts">
	// @ts-ignore
	import { NodeToolbar, Position } from '@xyflow/svelte'
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import type { FlowModule, FlowModuleValue } from '$lib/gen/types.gen'
	import { ClipboardCopy, GitBranchPlus } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { GraphModuleState } from '../../model'
	import { getStateColor } from '../../util'

	export let data: {
		offset: number
		value: FlowModuleValue
		module: FlowModule
		trigger: boolean
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

	$: idx = data.modules?.findIndex((m) => m.id === data.module.id)

	const dispatch = createEventDispatcher()
	let openMenu: boolean | undefined = undefined

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
		trigger={data.trigger}
		insertable={data.insertable}
		insertableEnd={data.insertableEnd}
		annotation={state?.flow_jobs
			? 'ierations ' + state?.flow_jobs?.length + '/' + (state?.iteration_total ?? '?')
			: ''}
		branchable={data.branchable}
		bgColor={getStateColor(type, darkMode)}
		modules={data.modules ?? []}
		moving={data.moving}
		duration_ms={data.duration_ms}
		disableAi={data.disableAi}
		wrapperId={data.wrapperId}
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

<NodeToolbar isVisible position={Position.Top} align="end">
	{#if data.insertable}
		<div
			class="{openMenu
				? 'z-20'
				: ''} w-[27px] absolute -top-[35px] left-[50%] right-[50%] -translate-x-1/2"
		>
			{#if data.moving}
				<button
					title="Add branch"
					on:click={() => {
						dispatch('insert', { modules: data.modules, index: idx, detail: 'move' })
					}}
					type="button"
					disabled={data.wrapperId === data.moving}
					class=" text-primary bg-surface border mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
				>
					<ClipboardCopy class="m-[5px]" size={15} />
				</button>
			{/if}
		</div>
	{/if}
</NodeToolbar>

<NodeToolbar isVisible position={Position.Bottom} align="center">
	{#if data.value.type === 'branchall' && data.insertable}
		<button
			class="rounded-full border hover:bg-surface-hover bg-surface p-1"
			on:click={() => {
				dispatch('addBranch')
			}}
		>
			<GitBranchPlus size={16} />
		</button>
	{/if}
</NodeToolbar>
