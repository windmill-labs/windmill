<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { GraphModuleState } from '../../model'

	export let data: {
		id: string
		insertable: boolean
		modules: FlowModule[]
		eventHandlers: GraphEventHandlers
		flowModuleStates: Record<string, GraphModuleState> | undefined
		offset: number
	}
</script>

<NodeWrapper offset={data.offset} let:darkMode enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={'Collect result from all branches'}
		id={data.id}
		selectable={true}
		selected={false}
		bgColor={getStateColor(undefined, darkMode)}
		bgHoverColor={getStateHoverColor(undefined, darkMode)}
		borderColor={getStateColor(data?.flowModuleStates?.[data?.id]?.type, darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
