<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen/models/FlowModule'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStateColor } from '../../util'
	import type { Writable } from 'svelte/store'
	import { getContext } from 'svelte'

	export let data: {
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
		success: boolean | undefined
	}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper let:darkMode enableSourceHandle={false}>
	<VirtualItem
		id={'Result'}
		label={'Result'}
		modules={undefined}
		index={data.modules.length}
		selectable={true}
		selected={$selectedId === 'Result'}
		insertable={false}
		hideId={true}
		bgColor={getStateColor(
			data.success == undefined ? undefined : data.success ? 'Success' : 'Failure',
			darkMode
		)}
		on:select={(e) => {
			data.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
