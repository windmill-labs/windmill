<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let node: DecisionTreeNode

	let selected = false

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
	<div class="absolute -bottom-10 left-1/2 transform -translate-x-1/2 flex items-center">
		<Button
			color="light"
			startIcon={{ icon: Plus }}
			size="xs2"
			variant="border"
			btnClasses="w-6 h-6 rounded-full p-0"
			on:click={() => {
				dispatch('add', node.id)
			}}
		/>
	</div>
</div>
