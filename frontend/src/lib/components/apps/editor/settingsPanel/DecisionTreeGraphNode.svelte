<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import type { DecisionTreeNode } from '../component'
	import Button from '$lib/components/common/button/Button.svelte'

	export let node: DecisionTreeNode

	const dispatch = createEventDispatcher()
</script>

<button
	class="p-2 bg-surface w-full h-8 relative"
	on:click={() => {
		dispatch('select', node.id)
	}}
>
	{node.label}

	<div class="absolute right-0 top-0 bottom-0 flex items-center">
		<Button />
	</div>
</button>
