<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button, Badge } from '../common'
	import Modal from '../common/modal/Modal.svelte'
	import Portal from '../Portal.svelte'


	interface Props {
		open?: boolean;
		inputs?: string[];
	}

	let { open = $bindable(false), inputs = [] }: Props = $props();

	const dispatch = createEventDispatcher()
</script>

<Portal>
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

		{#snippet actions()}
				<Button
				
				on:click={() => {
					open = false
					dispatch('confirmed')
				}}
				color="light"
				size="sm"
			>
				<span class="inline-flex gap-2">Add <Badge color="dark-green">Enter</Badge></span>
			</Button>
			{/snippet}
	</Modal>
</Portal>
