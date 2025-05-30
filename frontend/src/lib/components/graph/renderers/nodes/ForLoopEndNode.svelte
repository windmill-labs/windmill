<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { GraphModuleState } from '../../model'
	import { getStateColor } from '../../util'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'

	export let data: {
		offset: number
		id: string
		insertable: boolean
		flowModuleStates: Record<string, GraphModuleState> | undefined
		eventHandlers: GraphEventHandlers
		simplifiedTriggerView: boolean
	}
</script>

<NodeWrapper offset={data.offset} let:darkMode>
	{#if data.simplifiedTriggerView}
		<VirtualItem
			label={'Each event is processed'}
			selectable={false}
			selected={false}
			id={data.id}
			hideId
			bgColor={getStateColor(undefined, darkMode)}
			borderColor={getStateColor(data.flowModuleStates?.[data.id]?.type, darkMode)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{:else}
		<VirtualItem
			label={'Collect result of each iteration'}
			selectable={true}
			selected={false}
			id={data.id}
			bgColor={getStateColor(undefined, darkMode)}
			borderColor={getStateColor(data.flowModuleStates?.[data.id]?.type, darkMode)}
			on:select={(e) => {
				setTimeout(() => data?.eventHandlers?.select(e.detail))
			}}
		/>
	{/if}
</NodeWrapper>
