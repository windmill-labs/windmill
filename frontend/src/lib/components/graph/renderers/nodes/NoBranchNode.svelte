<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStateColor } from '../../util'
	import type { GraphModuleState } from '../../model'

	export let data: {
		id: string
		insertable: boolean
		modules: FlowModule[]
		eventHandlers: GraphEventHandlers
		flowModuleStates: Record<string, GraphModuleState> | undefined
		offset: number
		label: string | undefined
	}
</script>

<NodeWrapper offset={data.offset} let:darkMode enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={data.label ?? 'No branches'}
		modules={data.modules}
		id={data.id}
		hideId={true}
		selectable={true}
		selected={false}
		insertable={false}
		bgColor={getStateColor(undefined, darkMode)}
		borderColor={getStateColor(data?.flowModuleStates?.[data?.id]?.type, darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
