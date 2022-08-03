<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { createPopperActions } from 'svelte-popperjs'
	import PropPicker from './PropPicker.svelte'

	const [popperRef, popperContent] = createPopperActions({
		placement: 'bottom-end',
		strategy: 'fixed'
	})

	export let pickableProperties: Object | undefined
	export let disabled = false
	let isOpen = false

	function toggle() {
		isOpen = !isOpen
	}

	const dispatch = createEventDispatcher()
</script>

{#if !disabled}
	<div class="w-full">
		<div use:popperRef on:click={toggle}>
			<slot />
		</div>

		{#if isOpen}
			<div class="content" use:popperContent>
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
