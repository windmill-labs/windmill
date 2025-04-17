<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStateColor, getStateHoverColor } from '../../util'
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
		selectable={true}
		selected={$selectedId === 'Result'}
		hideId={true}
		bgColor={getStateColor(
			data.success == undefined ? undefined : data.success ? 'Success' : 'Failure',
			darkMode
		)}
		bgHoverColor={getStateHoverColor(
			data.success == undefined ? undefined : data.success ? 'Success' : 'Failure',
			darkMode
		)}
		on:select={(e) => {
			setTimeout(() => data?.eventHandlers?.select(e.detail))
		}}
	/>
</NodeWrapper>
