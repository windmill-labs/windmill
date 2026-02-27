<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button, Badge } from '../common'
	import Modal from '../common/modal/Modal.svelte'
	import Portal from '../Portal.svelte'

	interface Props {
		open?: boolean
		inputs?: string[]
		onconfirmed?: (...args: any[]) => any
		oncanceled?: (...args: any[]) => any
	}

	let { open = $bindable(false), inputs = [], onconfirmed = undefined,
		oncanceled = undefined }: Props = $props()

	const dispatch = createEventDispatcher()
</script>

<Portal>
	<Modal
		bind:open
		onconfirmed={() => {
			open = false
			dispatch('confirmed')
			onconfirmed?.()
		}}
		oncanceled={oncanceled}
		title="Windmill AI wants to add the following inputs to the flow:"
	>
		<ul class=" list-disc pl-5">
			{#each inputs as input}
				<li>{input}</li>
			{/each}
		</ul>

		{#snippet actions()}
			<Button
				onclick={() => {
					open = false
					dispatch('confirmed')
					onconfirmed?.()
				}}
				color="light"
				size="sm"
			>
				<span class="inline-flex gap-2">Add <Badge color="dark-green">Enter</Badge></span>
			</Button>
		{/snippet}
	</Modal>
</Portal>
