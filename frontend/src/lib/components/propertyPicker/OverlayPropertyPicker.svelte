<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Overlay from 'svelte-overlay'
	import PropPicker from './PropPicker.svelte'
	import WarningMessage from './WarningMessage.svelte'

	export let previousSchema: Object | undefined
	let isOpen = false

	const dispatch = createEventDispatcher()

	function handleWindowKeyDown(event: { key: string }) {
		if (event.key === 'Escape') {
			isOpen = false
		}
	}

	function onMouseLeave(mouseEvent: MouseEvent) {
		const { offsetY } = mouseEvent
		const target = mouseEvent.target as HTMLInputElement

		const down = offsetY >= target.clientHeight
		const up = offsetY <= 0

		const content = document.getElementsByClassName('content')
		const overlayBottom = content.item(0)?.classList.contains('bottom-right')

		if ((down && overlayBottom) || (up && !overlayBottom)) {
			return
		}
		isOpen = !isOpen
	}
</script>

<Overlay
	onWindowKeyDown={handleWindowKeyDown}
	closeOnClickOutside
	closeOnScroll
	bind:isOpen
	class="w-full"
	style="z-index:unset"
>
	<div
		slot="parent"
		let:toggle
		on:mousemove={() => !isOpen && toggle()}
		on:mouseleave={onMouseLeave}
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
