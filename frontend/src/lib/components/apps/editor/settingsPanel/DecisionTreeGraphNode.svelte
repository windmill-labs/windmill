<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import InsertDecisionTreeNode from './decisionTree/InsertDecisionTreeNode.svelte'
	import type { Writable } from 'svelte/store'
	import { Badge } from '$lib/components/common'

	export let node: DecisionTreeNode
	export let selected = false
	export let editable: boolean = true

	let open: boolean = false

	const dispatch = createEventDispatcher()

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
		<div class="grow text-xs font-normal"> Lorem ipsum </div>
		<Badge color="indigo">
			{node.label}
		</Badge>
	</Button>
	{#if node.id !== 'end' && editable}
		<div
			class={twMerge(
				'absolute -bottom-10 left-1/2 transform -translate-x-1/2 flex items-center',
				open ? 'z-20' : ''
			)}
		>
			<InsertDecisionTreeNode bind:open on:node on:branch canBranch={node.next.length > 1} />
		</div>
	{/if}
</div>
