<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button } from '$lib/components/common'
	import { Plug } from 'lucide-svelte'

	interface Props {
		label?: string
	}

	let { label = 'Add object from an expression' }: Props = $props()

	const dispatch = createEventDispatcher()

	function handleClick() {
		dispatch('click')
	}
</script>

<div class="mt-2 mb-2">
	<Button
		variant="border"
		color="light"
		size="xs"
		startIcon={{ icon: Plug }}
		on:click={handleClick}
	>
		{label}
	</Button>
</div>
