<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	// @ts-ignore

	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'

	type $$Props = NodeProps

	export let data: $$Props['data']
</script>

{#if data.time}
	<NodeToolbar isVisible position={Position.Top} align="end">
		<span class="text-xs">0.01s</span>
	</NodeToolbar>
{/if}

<div style={`margin-left: ${data.offset}px;`}>
	<MapItem mod={data.module} modules={[]} />
</div>

<Handle type="source" position={Position.Bottom} style={`margin-left: ${data.offset / 2}px;`} />
<Handle type="target" position={Position.Top} style={`margin-left: ${data.offset / 2}px;`} />
