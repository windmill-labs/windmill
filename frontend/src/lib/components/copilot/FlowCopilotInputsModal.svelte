<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button, Badge } from '../common'
	import Modal from '../common/modal/Modal.svelte'

	export let open = false

	export let inputs: string[] = []

	const dispatch = createEventDispatcher()
</script>

<Modal
	bind:open
	on:confirmed={() => {
		open = false
		dispatch('confirmed')
	}}
	on:canceled
	title="Windmill AI wants to add the following inputs to the flow:"
>
	<ul class=" list-disc pl-5">
		{#each inputs as input}
			<li>{input}</li>
		{/each}
	</ul>

	<Button
		slot="actions"
		on:click={() => {
			open = false
			dispatch('confirmed')
		}}
		color="light"
		size="sm"
	>
		<span class="inline-flex gap-2">Add <Badge color="dark-green">Enter</Badge></span>
	</Button>
</Modal>
