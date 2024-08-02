<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import { Wand2 } from 'lucide-svelte'

	type $$Props = NodeProps
	export let data: $$Props['data']
</script>

<NodeToolbar isVisible position={Position.Top}>
	<button class="rounded-full border bg-indigo-100 p-2 hover:bg-indigo-200">
		<Wand2 size={16} />
	</button>
</NodeToolbar>

<VirtualItem label="Input" modules={[]} index={1} selectable selected={false} insertable={false} />

<Handle type="source" position={Position.Bottom} />
