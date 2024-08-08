<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { FlowModule } from '$lib/gen/models/FlowModule'

	export let data: {
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

<NodeWrapper let:darkMode enableSourceHandle enableTargetHandle>
	<VirtualItem
		label={'Collect result from all branches'}
		modules={[]}
		index={data.modules.findIndex((m) => m.id == data.id) + 1}
		id={data.id}
		selectable
		selected={false}
		insertable={false}
		bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
	/>
</NodeWrapper>
