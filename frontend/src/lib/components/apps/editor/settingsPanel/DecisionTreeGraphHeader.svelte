<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import { twMerge } from 'tailwind-merge'
	import InsertDecisionTreeNode from './decisionTree/InsertDecisionTreeNode.svelte'
	import { X } from 'lucide-svelte'

	export let node: DecisionTreeNode
	export let editable: boolean = true
	export let canDelete: boolean = true
	export let label: string = ''

	let open: boolean = false

	const dispatch = createEventDispatcher<{
		nodeInsert: void
		delete: void
		addBranch: void
		removeBranch: void
	}>()
</script>

<div class="relative w-[274px] h-full">
	<div
		class={twMerge(
			'w-full h-full border border-black',
			'flex flex-row gap-2 items-center justify-between'
		)}
		style="background-color: rgb(223, 230, 238);"
	>
		<div class="grow text-xs font-normal"> {label} </div>
	</div>

	{#if canDelete}
		<button
			class="absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-primary
border-[1.5px] border-gray-700 bg-surface duration-150 hover:bg-red-400 hover:text-white
hover:border-red-700"
			on:click|preventDefault|stopPropagation={() => dispatch('removeBranch')}
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
				on:node={() => dispatch('nodeInsert')}
				on:addBranch={() => dispatch('addBranch')}
				canAddBranch={false}
			/>
		</div>
	{/if}
</div>
