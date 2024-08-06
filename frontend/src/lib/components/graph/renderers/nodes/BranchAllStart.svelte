<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let data: {
		label: string
		branchIndex: number
	}

	const dispatch = createEventDispatcher()
</script>

<NodeToolbar isVisible position={Position.Top}>
	<button
		class="rounded-full border bg-white p-1 hover:bg-gray-200"
		on:click={() => {
			dispatch('deleteBranch', { branchIndex: data.branchIndex })
		}}
	>
		<X size={16} />
	</button>
</NodeToolbar>

<NodeWrapper>
	<svelte:fragment slot="header">
		<span class="text-xs">0.01s</span>
	</svelte:fragment>
	<VirtualItem
		label={data.label}
		modules={[]}
		index={1}
		selectable
		selected={false}
		insertable={false}
	/>
</NodeWrapper>
