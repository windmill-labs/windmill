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

	type PickerVariation = 'append' | 'connect'
	let pickerVariation: PickerVariation = 'append'

	export function unfocus() {
		isFocused = false
		close()
	}
	export function focus(newPickerVariation?: PickerVariation) {
		if (newPickerVariation) {
			pickerVariation = newPickerVariation
		}
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
		pickerVariation = 'append'
		timeout = setTimeout(() => (isOpen = false), 50)
	}

	const dispatch = createEventDispatcher()
</script>

{#if !disabled}
	<div class="w-full">
		<div use:popperRef>
			<slot />
		</div>

		{#if isOpen}
			<div class="content" use:popperContent on:mouseenter={open} on:mouseleave={close}>
				<PropPicker
					bind:pickableProperties
					on:select={(event) => {
						dispatch('select', { propPath: event.detail, pickerVariation })
						isOpen = false
						pickerVariation = 'append'
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
