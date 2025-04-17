<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { GraphModuleState } from '../../model'

	export let data: {
		id: string
		insertable: boolean
		eventHandlers: GraphEventHandlers
		flowModuleStates: Record<string, GraphModuleState> | undefined
		offset: number
	}
</script>

<NodeWrapper let:darkMode offset={data.offset}>
	<VirtualItem
		label={'Collect result from chosen branch'}
		id={data.id}
		selectable={true}
		selected={false}
		bgColor={getStateColor(undefined, darkMode)}
		bgHoverColor={getStateHoverColor(undefined, darkMode)}
		borderColor={getStateColor(data?.flowModuleStates?.[data?.id]?.type, darkMode)}
		on:select={(e) => {
			setTimeout(() => data?.eventHandlers?.select(e.detail))
		}}
	/>
</NodeWrapper>
