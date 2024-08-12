<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStateColor } from '../../util'

	export let data: {
		id: string
		insertable: boolean
		modules: FlowModule[]
		eventHandlers: GraphEventHandlers
	}
</script>

<NodeWrapper let:darkMode enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={'Collect result from all branches'}
		modules={data.modules}
		index={data.modules.findIndex((m) => m.id == data.id) + 1}
		id={data.id}
		selectable={true}
		selected={false}
		insertable={false}
		bgColor={getStateColor(undefined, darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
