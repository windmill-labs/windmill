<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import type { FlowStatusModule } from '$lib/gen'
	import type { GraphModuleState } from '../../model'
	import { getStateColor } from '../../util'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import type { GraphEventHandlers } from '../../graphBuilder'

	export let data: {
		label: string
		branchIndex: number
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
		id: string
		modules: FlowModule[]
		selected: boolean
		eventHandlers: GraphEventHandlers
	}

	const dispatch = createEventDispatcher()

	let borderStatus: FlowStatusModule['type'] | undefined = undefined

	let flow_jobs_success = data?.flowModuleStates?.[data?.id]?.flow_jobs_success
	if (!flow_jobs_success) {
		borderStatus = 'WaitingForPriorSteps'
	} else {
		let status = flow_jobs_success?.[data.branchIndex]
		if (status == undefined) {
			borderStatus = 'WaitingForExecutor'
		} else {
			borderStatus = status ? 'Success' : 'Failure'
		}
	}
</script>

<NodeToolbar isVisible position={Position.Top}>
	{#if data.insertable}
		<Popover>
			<button
				class="rounded-full border p-1 hover:bg-surface-hover bg-surface"
				on:click={() => {
					dispatch('deleteBranch', { branchIndex: data.branchIndex })
				}}
			>
				<X size={16} />
			</button>

			<svelte:fragment slot="text">Delete branch</svelte:fragment>
		</Popover>
	{/if}
</NodeToolbar>

<NodeWrapper let:darkMode>
	<VirtualItem
		label={data.label}
		modules={data.modules}
		index={data.branchIndex}
		selectable
		selected={data.selected}
		insertable={data.insertable}
		bgColor={getStateColor(undefined, darkMode)}
		borderColor={borderStatus
			? getStateColor(borderStatus, darkMode) + (!darkMode ? '; border-width: 3px' : '')
			: undefined}
		on:select={() => {
			data.eventHandlers.select(data.id)
		}}
		on:deleteBranch={(e) => {
			data.eventHandlers.deleteBranch(e.detail, data.label)
		}}
		on:insert={(e) => {
			data.eventHandlers.insert(e.detail)
		}}
	/>
</NodeWrapper>
