<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { GraphModuleState } from '../../model'
	import { computeBorderStatus } from '../utils'

	export let data: {
		id: string
		insertable: boolean
		modules: FlowModule[]
		eventHandlers: GraphEventHandlers
		flowModuleStates: Record<string, GraphModuleState> | undefined
		offset: number
		label: string | undefined
		branchOne: boolean
	}

	$: borderStatus = data.branchOne
		? computeBorderStatus(0, 'branchone', data.flowModuleStates?.[data.id])
		: undefined
</script>

<NodeWrapper offset={data.offset} let:darkMode enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={data.label ?? 'No branches'}
		id={data.id}
		hideId={true}
		selectable={true}
		selected={false}
		bgColor={getStateColor(undefined, darkMode)}
		bgHoverColor={getStateHoverColor(undefined, darkMode)}
		borderColor={getStateColor(borderStatus, darkMode)}
		on:select={(e) => {
			setTimeout(() => data?.eventHandlers?.select(e.detail))
		}}
	/>
</NodeWrapper>
