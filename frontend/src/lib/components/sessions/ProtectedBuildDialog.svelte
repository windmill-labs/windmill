<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'

	// Asked when a Home draft on a protected workspace leaves Use: the session builds in a new
	// fork, or, for a user the rule lets through, in the workspace itself.
	let {
		isOpen = $bindable(),
		workspace,
		canFork,
		canDirect,
		onFork,
		onDirect
	}: {
		isOpen: boolean
		workspace: string
		canFork: boolean
		canDirect: boolean
		onFork: () => void
		onDirect: () => void
	} = $props()

	function pick(choice: () => void) {
		isOpen = false
		choice()
	}
</script>

<Modal2
	title="{workspace} is protected"
	target="#content"
	fixedWidth="xs"
	fixedHeight="adaptive"
	contentClasses="flex-col gap-4"
	bind:isOpen
>
	<p class="text-xs text-secondary">
		Direct changes to {workspace} are turned off.
		{#if canFork}
			Build in a new fork instead: it is created when you send your first message, and you deploy
			your changes back from it.
		{/if}
		{#if canDirect}
			Its protection lets you through, so you can also work in {workspace} directly.
		{/if}
	</p>
	<div class="flex justify-end gap-2">
		<Button variant="subtle" unifiedSize="md" onClick={() => (isOpen = false)}>Cancel</Button>
		{#if canDirect}
			<Button variant="default" unifiedSize="md" onClick={() => pick(onDirect)}>
				Work in {workspace} directly
			</Button>
		{/if}
		{#if canFork}
			<Button variant="accent" unifiedSize="md" onClick={() => pick(onFork)}>Work in a fork</Button>
		{/if}
	</div>
</Modal2>
