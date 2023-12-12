<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import InsertDecisionTreeNode from './decisionTree/InsertDecisionTreeNode.svelte'
	import type { Writable } from 'svelte/store'
	import { Badge } from '$lib/components/common'
	import { X } from 'lucide-svelte'

	export let node: DecisionTreeNode
	export let selected = false
	export let editable: boolean = true
	export let isHead: boolean = false
	export let canDelete: boolean = true

	let open: boolean = false

	const dispatch = createEventDispatcher<{
		select: string
		nodeInsert: void
		branchInsert: void
		delete: void
		addBranch: void
	}>()

	const { selectedNodeId } = getContext<{
		selectedNodeId: Writable<string | undefined>
	}>('DecisionTreeEditor')

	$: selected = $selectedNodeId === node.id
</script>

<div class="relative w-[274px]">
	<Button
		class={twMerge(
			'p-2 bg-surface w-full h-8 relative rounded-md',
			selected ? 'outline outline-2 outline-offset-2' : '',
			'flex flex-row gap-2 items-center justify-between'
		)}
		on:click={() => {
			selected = true
			dispatch('select', node.id)
		}}
	>
		<div class="grow text-xs font-normal"> {node.label} </div>
		<Badge color="indigo">
			{node.id}
		</Badge>
	</Button>

	{#if canDelete}
		<button
			class={twMerge(
				'absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-primary',
				'border-[1.5px] border-gray-700 bg-surface duration-150 hover:bg-red-400 hover:text-white hover:border-red-700',
				selected ? '' : '!hidden'
			)}
			on:click|preventDefault|stopPropagation={() => dispatch('delete')}
		>
			<X class="mx-[3px]" size={14} strokeWidth={2} />
		</button>
	{/if}

	{#if node.id !== 'end' && editable}
		<div
			class={twMerge(
				'absolute -bottom-10 left-1/2 transform -translate-x-1/2 flex items-center',
				open ? 'z-20' : ''
			)}
		>
			<InsertDecisionTreeNode
				bind:open
				on:node={() => {
					dispatch('nodeInsert')
				}}
				on:branch={() => {
					dispatch('branchInsert')
				}}
				on:addBranch={() => {
					dispatch('addBranch')
				}}
				canBranch={node.next.length > 1}
				canInsertBranch={!isHead}
			/>
		</div>
	{/if}
</div>
