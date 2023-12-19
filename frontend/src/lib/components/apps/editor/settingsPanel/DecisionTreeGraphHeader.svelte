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

<div class="relative h-full">
	<div
		class={twMerge(
			'w-full h-full border border-gray-400',
			'flex flex-row gap-2 items-center justify-between rounded-sm overflow-hidden'
		)}
		style="min-width: 275px; max-height: 80px; background-color: {document.documentElement.classList.contains(
			'dark'
		)
			? '#2e3440'
			: '#dfe6ee'}"
	>
		<div class="ml-4 text-xs font-normal text-primary"> {label} </div>
	</div>

	{#if canDelete}
		<div class="w-[27px] absolute -top-[32px] left-[50%] right-[50%] -translate-x-1/2">
			<button
				title="Delete branch"
				on:click|preventDefault|stopPropagation={() => dispatch('removeBranch')}
				type="button"
				class="text-primary bg-surface border mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
			>
				<X class="m-[5px]" size={15} />
			</button>
		</div>
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
