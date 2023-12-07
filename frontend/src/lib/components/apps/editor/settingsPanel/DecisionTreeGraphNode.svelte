<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import InsertDecisionTreeNode from './decisionTree/InsertDecisionTreeNode.svelte'

	export let node: DecisionTreeNode

	let selected = false

	let open: boolean = false

	const dispatch = createEventDispatcher()
</script>

<div class="relative w-[274px]">
	<Button
		class={twMerge(
			'p-2 bg-surface w-full h-8 relative rounded-md',
			selected ? 'outline outline-2 outline-offset-2' : ''
		)}
		on:click={() => {
			selected = true
			dispatch('select', node.id)
		}}
	>
		Tab: {node.label}
	</Button>
	<div
		class={twMerge(
			'absolute -bottom-10 left-1/2 transform -translate-x-1/2 flex items-center',
			open ? 'z-20' : ''
		)}
	>
		<InsertDecisionTreeNode bind:open on:node on:branch />
	</div>
</div>
