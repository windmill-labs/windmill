<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import type { FlowModule, FlowModuleValue } from '$lib/gen'
	import { GitBranchPlus, Maximize2 } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'
	import type { GraphModuleState } from '../../model'
	import { getStateColor, getStateHoverColor } from '../../util'

	interface Props {
		data: {
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
	}

	let { data }: Props = $props()

	let type = $derived(data.flowModuleStates?.[data.module.id]?.type)
	if (!type && data.flowJobs) {
		type = 'InProgress'
	}

	let moduleState = $derived(data.flowModuleStates?.[data.module.id])
	let flowJobs = $derived(
		moduleState?.flow_jobs
			? {
					flowJobs: moduleState?.flow_jobs,
					selected: moduleState?.selectedForloopIndex ?? 0,
					selectedManually: moduleState?.selectedForLoopSetManually,
					flowJobsSuccess: moduleState?.flow_jobs_success
				}
			: (undefined as any)
	)
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		{#if data.module.value.type == 'flow'}
			<button
				title="Expand subflow"
				class="z-50 absolute -top-[10px] right-[25px] rounded-full h-[20px] w-[20px] center-center text-primary bg-surface duration-0 hover:bg-surface-hover"
				onclick={stopPropagation(
					preventDefault(() => {
						if (data.module.value.type == 'flow') {
							data.eventHandlers.expandSubflow(data.module.id, data.module.value.path)
						}
					})
				)}
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
					((moduleState?.selectedForloopIndex ?? 0) >= 0
						? (moduleState?.selectedForloopIndex ?? 0) + 1
						: moduleState?.flow_jobs?.length) +
					'/' +
					(moduleState?.iteration_total ?? '?')
				: ''}
			bgColor={getStateColor(type, darkMode, true, moduleState?.skipped)}
			bgHoverColor={getStateHoverColor(type, darkMode, true, moduleState?.skipped)}
			moving={data.moving}
			duration_ms={moduleState?.duration_ms}
			retries={data.retries}
			{flowJobs}
			on:delete={(e) => {
				data.eventHandlers.delete(e.detail, '')
			}}
			on:insert={(e) => {
				console.log('insert', e.detail)
				data.eventHandlers.insert(e.detail)
			}}
			on:changeId={(e) => {
				data.eventHandlers.changeId(e.detail)
			}}
			on:move={(e) => {
				data.eventHandlers.move({ id: data.module.id })
			}}
			on:newBranch={(e) => {
				data.eventHandlers.newBranch(data.module.id)
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
					onclick={() => {
						data?.eventHandlers?.newBranch(data.module.id)
					}}
				>
					<GitBranchPlus size={16} />
				</button>
			{/if}
		</div>
	{/snippet}
</NodeWrapper>
