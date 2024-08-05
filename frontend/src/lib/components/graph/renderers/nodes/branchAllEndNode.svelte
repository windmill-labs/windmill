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

<VirtualItem
	label={'Collect result from all branches'}
	modules={[]}
	index={1}
	id={data.id}
	selectable
	selected={false}
	insertable={false}
/>

<Handle type="source" position={Position.Bottom} />
<Handle type="target" position={Position.Top} />
