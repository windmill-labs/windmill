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

	function onMouseLeave(mouseEvent: MouseEvent) {
		const target = mouseEvent.target as HTMLInputElement

		if (!target?.classList.contains('property-picker')) {
			isOpen = !isOpen
		}
	}
</script>

<Overlay
	onWindowKeyDown={handleWindowKeyDown}
	closeOnClickOutside
	closeOnScroll
	bind:isOpen
	class="w-full"
	zIndex={30 - 1 - index}
>
	<div
		slot="parent"
		let:toggle
		on:mousemove={() => !isOpen && toggle()}
		on:mouseleave={onMouseLeave}
		class="property-picker"
	>
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
