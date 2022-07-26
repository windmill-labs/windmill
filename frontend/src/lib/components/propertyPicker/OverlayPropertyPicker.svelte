<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { createPopperActions } from 'svelte-popperjs'
	import PropPicker from './PropPicker.svelte'

	const [popperRef, popperContent] = createPopperActions({
		placement: 'bottom',
		strategy: 'fixed'
	})

	export let pickableProperties: Object | undefined
	export let disabled = false
	let isOpen = false

	const dispatch = createEventDispatcher()

	function toggle() {
		isOpen = !isOpen
	}

	function onMouseLeave(mouseEvent: MouseEvent) {
		const { offsetY } = mouseEvent
		const target = mouseEvent.target as HTMLInputElement

		const down = offsetY >= target.clientHeight
		const up = offsetY <= 0

		const content = document.getElementsByClassName('content')
		const overlayBottom = content.item(0)?.getAttribute('data-popper-placement') === 'bottom'

		if ((down && overlayBottom) || (up && !overlayBottom)) {
			return
		}
		toggle()
	}
</script>

{#if !disabled}
	<div class="w-full">
		<div use:popperRef on:mousemove={() => !isOpen && toggle()} on:mouseleave={onMouseLeave}>
			<slot />
		</div>

		{#if isOpen}
			<div class="content" use:popperContent on:mouseleave={toggle}>
				<PropPicker
					bind:pickableProperties
					on:select={(event) => {
						isOpen = false
						dispatch('select', event.detail)
					}}
				/>
			</div>
		{/if}
	</div>
{:else}
	<slot />
{/if}

<style>
	.content {
		@apply drop-shadow-xl;
		@apply w-full;
		@apply max-w-4xl;

		@apply px-6;
		@apply z-50;
	}
</style>
