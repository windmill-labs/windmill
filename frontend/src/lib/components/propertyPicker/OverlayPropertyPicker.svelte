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
	let isFocused = false

	let timeout: NodeJS.Timeout

	export function unfocus() {
		isFocused = false
		close()
	}
	export function focus() {
		isFocused = true
		open()
	}

	function open() {
		if (isFocused) {
			clearTimeout(timeout)
			!isOpen && (isOpen = true)
		}
	}
	function close() {
		timeout = setTimeout(() => (isOpen = false), 200)
	}

	const dispatch = createEventDispatcher()
</script>

{#if !disabled}
	<div class="w-full">
		<div use:popperRef on:mouseenter={open} on:mouseleave={close}>
			<slot />
		</div>

		{#if isOpen}
			<div class="content" use:popperContent on:mouseenter={open} on:mouseleave={close}>
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
