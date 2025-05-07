<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import type { GraphModuleState } from '../../model'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { computeBorderStatus } from '../utils'

	export let data: {
		label: string
		branchIndex: number
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
		id: string
		modules: FlowModule[]
		selected: boolean
		eventHandlers: GraphEventHandlers
		offset: number
	}

	$: borderStatus = computeBorderStatus(
		data.branchIndex,
		'branchall',
		data.flowModuleStates?.[data.id]
	)
</script>

<NodeWrapper let:darkMode offset={data.offset}>
	<VirtualItem
		label={data.label}
		selectable
		selected={data.selected}
		bgColor={getStateColor(undefined, darkMode)}
		bgHoverColor={getStateHoverColor(undefined, darkMode)}
		borderColor={borderStatus
			? getStateColor(borderStatus, darkMode) + (!darkMode ? '; border-width: 3px' : '')
			: undefined}
		on:select={() => {
			setTimeout(() => data.eventHandlers.select(data.id))
		}}
		on:insert={(e) => {
			setTimeout(() => data.eventHandlers.insert(e.detail))
		}}
	/>
	{#if data.insertable}
		<button
			title="Delete branch"
			class="z-50 absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] center-center text-primary
outline-[1px] outline outline-gray-700 bg-surface duration-0 hover:bg-red-400 hover:text-white
hover:border-red-700"
			on:click|preventDefault|stopPropagation={() => {
				data.eventHandlers.deleteBranch(
					{
						module: data.modules.find((m) => m.id === data.id),
						index: data.branchIndex
					},
					data.label
				)
			}}
		>
			<X size={12} />
		</button>
	{/if}
</NodeWrapper>
