<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'

	export let data: {
		label: string
	}
</script>

<NodeToolbar isVisible position={Position.Top}>
	<div style="width:300px;" class="flex flex-row">
		<span class="text-xs">0.01s</span>
	</div>
</NodeToolbar>

<VirtualItem
	label={data.label}
	modules={[]}
	index={1}
	selectable
	selected={false}
	insertable={false}
/>

<Handle type="source" position={Position.Bottom} />
<Handle type="target" position={Position.Top} />
