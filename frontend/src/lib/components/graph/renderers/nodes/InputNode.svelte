<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import { Wand2 } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let data: {
		insertable: boolean
	}
</script>

{#if data.insertable}
	<NodeToolbar isVisible position={Position.Top}>
		<Popover>
			<button class="rounded-full border p-2 hover:bg-surface-hover bg-surface text-indigo-400">
				<Wand2 size={16} />
			</button>

			<svelte:fragment slot="text">AI Flow builder</svelte:fragment>
		</Popover>
	</NodeToolbar>
{/if}

<NodeWrapper enableTargetHandle={false} let:darkMode>
	<VirtualItem
		label="Input"
		modules={[]}
		index={1}
		selectable
		selected={false}
		insertable={false}
		bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
	/>
</NodeWrapper>
