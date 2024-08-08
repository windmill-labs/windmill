<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let data: {
		label: string
		branchIndex: number
		insertable: boolean
	}

	const dispatch = createEventDispatcher()
</script>

<NodeToolbar isVisible position={Position.Top}>
	{#if data.insertable}
		<Popover>
			<button
				class="rounded-full border p-1 hover:bg-surface-hover bg-surface"
				on:click={() => {
					dispatch('deleteBranch', { branchIndex: data.branchIndex })
				}}
			>
				<X size={16} />
			</button>

			<svelte:fragment slot="text">Delete branch</svelte:fragment>
		</Popover>
	{/if}
</NodeToolbar>

<NodeWrapper let:darkMode>
	<VirtualItem
		label={data.label}
		modules={[]}
		index={data.branchIndex}
		selectable
		selected={false}
		insertable={data.insertable}
		bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
	/>
</NodeWrapper>
