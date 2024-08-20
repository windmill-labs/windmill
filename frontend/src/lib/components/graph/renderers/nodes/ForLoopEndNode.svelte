<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphModuleState } from '../../model'
	import { getStateColor } from '../../util'
	import type { GraphEventHandlers } from '../../graphBuilder'

	export let data: {
		offset: number
		id: string
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
		eventHandlers: GraphEventHandlers
	}
</script>

<NodeWrapper offset={data.offset} let:darkMode>
	<VirtualItem
		label={'Collect result of each iteration'}
		selectable={true}
		selected={false}
		id={data.id}
		bgColor={getStateColor(undefined, darkMode)}
		borderColor={getStateColor(data.flowModuleStates?.[data.id]?.type, darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
