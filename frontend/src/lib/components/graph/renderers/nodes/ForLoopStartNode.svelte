<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'

	export let data: {
		offset: number
		id: string
		time: number | undefined
	}
</script>

{#if data.time}
	<NodeToolbar isVisible position={Position.Top} align="end">
		<span class="text-xs">0.01s</span>
	</NodeToolbar>
{/if}

<NodeWrapper let:darkMode offset={data.offset} enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={'Do one iteration'}
		modules={[]}
		index={1}
		selectable
		selected={false}
		insertable={false}
		id={data.id}
		bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
	/>
</NodeWrapper>
