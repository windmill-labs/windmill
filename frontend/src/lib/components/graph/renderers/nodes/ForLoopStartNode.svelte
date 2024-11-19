<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule, FlowStatusModule } from '$lib/gen'
	import { getStateColor } from '../../util'
	import type { GraphModuleState } from '../../model'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getContext } from 'svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'

	export let data: {
		offset: number
		id: string
		modules: FlowModule[]
		flowModuleStates: Record<string, GraphModuleState> | undefined
		eventHandlers: GraphEventHandlers
		simplifiedTriggerView: boolean
	}

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const pickablePropertiesFiltered = propPickerContext?.pickablePropertiesFiltered

	$: filteredInput = filterIterFromInput($pickablePropertiesFiltered?.flow_input)

	function filterIterFromInput(inputJson: Record<string, any> | undefined): Record<string, any> {
		if (!inputJson || typeof inputJson !== 'object' || !inputJson.iter) return {}
		return { iter: inputJson.iter }
	}

	function computeStatus(state: GraphModuleState | undefined): FlowStatusModule["type"] | undefined {
		if (state?.type == 'InProgress' || state?.type == 'Success' || state?.type == 'Failure') {
			let r = state?.flow_jobs_success?.[state?.selectedForloopIndex ?? 0] 
			if (r == undefined) return 'InProgress'
			return r ? 'Success' : 'InProgress'
		}
	}
</script>



<NodeWrapper let:darkMode offset={data.offset}>
	<VirtualItem
		label={data.simplifiedTriggerView ? 'For each new event' : 'Do one iteration'}
		selectable={false}
		selected={false}
		id={data.id}
		hideId
		bgColor={getStateColor(undefined, darkMode)}
		borderColor={getStateColor(computeStatus(data.flowModuleStates?.[data.id]), darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
		inputJson={filteredInput}
		prefix="flow_input"
	/>
</NodeWrapper>
