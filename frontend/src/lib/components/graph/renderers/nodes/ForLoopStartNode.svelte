<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import { getStateColor } from '../../util'
	import type { GraphModuleState } from '../../model'
	import type { GraphEventHandlers } from '../../graphBuilder'

	export let data: {
		offset: number
		id: string
		modules: FlowModule[]
		flowModuleStates: Record<string, GraphModuleState> | undefined
		eventHandlers: GraphEventHandlers
	}
</script>

<NodeWrapper let:darkMode offset={data.offset} enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={'Do one iteration'}
		modules={data.modules}
		selectable={false}
		insertable={false}
		selected={false}
		id={data.id}
		bgColor={getStateColor(undefined, darkMode)}
		borderColor={getStateColor(data.flowModuleStates?.[data.id]?.type, darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
