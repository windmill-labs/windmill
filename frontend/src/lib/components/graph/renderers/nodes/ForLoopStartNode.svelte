<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import { getStateColor } from '../../util'
	import type { GraphModuleState } from '../../model'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { TriggerContext } from '$lib/components/triggers'
	import { getContext } from 'svelte'

	export let data: {
		offset: number
		id: string
		modules: FlowModule[]
		flowModuleStates: Record<string, GraphModuleState> | undefined
		eventHandlers: GraphEventHandlers
	}

	const { viewSimplifiedTriggers } = getContext<TriggerContext>('TriggerContext')
	const currentModuleIndex = data.modules.findIndex((module) => module.id === data.id)
	const previousModule = currentModuleIndex > 0 ? data.modules[currentModuleIndex - 1] : undefined
	$: isLoopTrigger = previousModule?.isTrigger && $viewSimplifiedTriggers
</script>

<NodeWrapper let:darkMode offset={data.offset}>
	<VirtualItem
		label={isLoopTrigger ? 'For each event, do one iteration' : 'Do one iteration'}
		selectable={false}
		selected={false}
		id={data.id}
		hideId
		bgColor={getStateColor(undefined, darkMode)}
		borderColor={getStateColor(data.flowModuleStates?.[data.id]?.type, darkMode)}
		on:select={(e) => {
			data?.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
