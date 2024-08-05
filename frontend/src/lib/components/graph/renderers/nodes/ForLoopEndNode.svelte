<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'

	type $$Props = NodeProps

	export let data: $$Props['data']
	$$restProps
</script>

{#if data.time}
	<NodeToolbar isVisible position={Position.Top} align="end">
		<span class="text-xs">0.01s</span>
	</NodeToolbar>
{/if}
<div style={`margin-left: ${data.offset}px;`}>
	<VirtualItem
		label={'Collect result of each iteration'}
		modules={[]}
		id={data.id}
		index={1}
		selectable
		selected={false}
		insertable={false}
	/>
</div>
<Handle type="source" position={Position.Bottom} style={`margin-left: ${data.offset / 2}px;`} />
<Handle type="target" position={Position.Top} style={`margin-left: ${data.offset / 2}px;`} />
