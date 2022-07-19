<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Overlay from 'svelte-overlay'
	import PropPicker from './PropPicker.svelte'
	import WarningMessage from './WarningMessage.svelte'

	export let previousSchema: Object | undefined
	export let index: number
	let isOpen = false

	const dispatch = createEventDispatcher()

	function handleWindowKeyDown(event: { key: string }) {
		if (event.key === 'Escape') {
			isOpen = false
		}
	}
</script>

<Overlay
	onWindowKeyDown={handleWindowKeyDown}
	closeOnClickOutside
	closeOnScroll
	bind:isOpen
	class="w-full"
	zIndex={100 - index}
>
	<div slot="parent" let:toggle on:mouseenter={toggle}>
		<slot />
	</div>

	<div slot="content" class="content" let:toggle on:mouseleave={toggle} let:close>
		{#if Boolean(previousSchema)}
			<PropPicker
				props={previousSchema}
				on:select={(event) => {
					isOpen = false
					dispatch('select', event.detail)
				}}
			/>
		{:else}
			<WarningMessage {close} />
		{/if}
	</div>
</Overlay>

<style>
	.content {
		@apply w-full;
		@apply drop-shadow-xl;
	}
</style>
