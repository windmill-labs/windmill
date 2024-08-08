<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen/models/FlowModule'

	export let data: {
		offset: number
		id: string
		time: number | undefined
		insertable: boolean
		modules: FlowModule[]
	}
</script>

{#if data.time}
	<NodeToolbar isVisible position={Position.Top} align="end">
		<span class="text-xs">0.01s</span>
	</NodeToolbar>
{/if}

<NodeWrapper offset={data.offset} enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={'Collect result of each iteration'}
		modules={data.modules}
		index={1}
		selectable
		selected={false}
		insertable={data.insertable}
	/>
</NodeWrapper>
