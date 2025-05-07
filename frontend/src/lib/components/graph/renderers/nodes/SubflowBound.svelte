<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Minimize2 } from 'lucide-svelte'
	import type { GraphModuleState } from '../../model'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'

	export let data: {
		label: string
		preLabel: string | undefined
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
		subflowId: string
		id: string
		modules: FlowModule[]
		selected: boolean
		eventHandlers: GraphEventHandlers
		offset: number
	}
</script>

<NodeWrapper let:darkMode offset={data.offset}>
	<VirtualItem
		label={data.label}
		preLabel={data.preLabel}
		selectable
		selected={data.selected}
		bgColor={getStateColor(undefined, darkMode)}
		bgHoverColor={getStateHoverColor(undefined, darkMode)}
		borderColor={undefined}
		on:select={() => {
			setTimeout(() => data.eventHandlers?.select(data.id))
		}}
	/>
	<button
		title="Unexpand subflow"
		class="z-50 absolute -top-[10px] right-[25px] rounded-full h-[20px] w-[20px] center-center text-primary bg-surface duration-0 hover:bg-surface-hover"
		on:click|preventDefault|stopPropagation={() => {
			data.eventHandlers.minimizeSubflow(data.subflowId)
		}}
	>
		<Minimize2 size={12} />
	</button>
</NodeWrapper>
