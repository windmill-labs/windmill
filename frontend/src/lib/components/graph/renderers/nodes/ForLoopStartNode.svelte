<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'

	type $$Props = NodeProps

	export let data: $$Props['data']
	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

{#if data.time}
	<NodeToolbar isVisible position={Position.Top} align="end">
		<span class="text-xs">0.01s</span>
	</NodeToolbar>
{/if}

<div style={`margin-left: ${data.offset}px;`}>
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
</div>

<Handle type="source" position={Position.Bottom} style={`margin-left: ${data.offset / 2}px;`} />
<Handle type="target" position={Position.Top} style={`margin-left: ${data.offset / 2}px;`} />
