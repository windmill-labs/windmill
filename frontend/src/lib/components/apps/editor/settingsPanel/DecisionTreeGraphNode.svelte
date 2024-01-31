<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import InsertDecisionTreeNode from './decisionTree/InsertDecisionTreeNode.svelte'
	import type { Writable } from 'svelte/store'
	import { Badge } from '$lib/components/common'
	import { X } from 'lucide-svelte'
	import { getStateColor } from '$lib/components/graph'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let node: DecisionTreeNode
	export let selected = false
	export let canDelete: boolean = true
	export let canAddBranch: boolean = true
	export let index: number

	let open: boolean = false

	const dispatch = createEventDispatcher<{
		select: string
		nodeInsert: void
		delete: void
		addBranch: void
	}>()

	const { selectedNodeId } = getContext<{
		selectedNodeId: Writable<string | undefined>
	}>('DecisionTreeEditor')

	$: selected = $selectedNodeId === node.id
</script>

<div class="relative rounded-sm group">
	<Button
		class={twMerge(
			'p-2 bg-surface w-full h-8 relative rounded-sm border border-gray-400',
			selected ? 'outline outline-2 outline-offset-2 outline-gray-600' : '',
			'flex flex-row gap-2 items-center justify-between'
		)}
		style="width: 275px; height: 34px; background-color: {getStateColor(undefined)};"
		on:click={() => {
			selected = true
			dispatch('select', node.id)
		}}
	>
		<div class="ml-2 text-xs font-normal text-primary truncate">
			{node.label === '' ? `Tab: ${node.id}` : node.label}
		</div>
		<Badge color="indigo" small>
			Tab index: {index}
			<Tooltip>
				You can manually select a node using the <b>setTab</b> function with this index in a frontend
				runnable.
			</Tooltip>
		</Badge>
	</Button>

	{#if canDelete}
		<button
			class={twMerge(
				'absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-primary',
				'border-[1.5px] border-gray-700 bg-surface duration-150 hover:bg-red-400 hover:text-white hover:border-red-700',
				'group-hover:opacity-100 opacity-0'
			)}
			on:click|preventDefault|stopPropagation={() => dispatch('delete')}
		>
			<X class="mx-[3px]" size={14} strokeWidth={2} />
		</button>
	{/if}

	{#if node.id !== 'end'}
		<div
			class={twMerge(
				'absolute -bottom-10 left-1/2 transform -translate-x-1/2 flex items-center',
				open ? 'z-20' : ''
			)}
		>
			<InsertDecisionTreeNode
				on:node={() => dispatch('nodeInsert')}
				on:addBranch={() => dispatch('addBranch')}
				canAddBranch={canAddBranch || node.next.length > 1}
				canAddNode={node.next.length <= 1}
			/>
		</div>
	{/if}
</div>
