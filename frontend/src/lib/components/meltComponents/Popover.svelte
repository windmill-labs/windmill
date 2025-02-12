<script lang="ts">
	import { createPopover, createSync, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import { zIndexes } from '$lib/zIndexes'

	export let closeButton: boolean = false
	export let displayArrow: boolean = false
	export let placement: Placement = 'bottom'
	export let disablePopup: boolean = false
	export let openOnHover: boolean = false

	const {
		elements: { trigger, content, arrow, close: closeElement },
		states
	} = createPopover({
		forceVisible: true,
		positioning: {
			placement
		}
	})

	let isOpen = false
	const sync = createSync(states)
	$: sync.open(isOpen, (v) => (isOpen = v))

	export function close() {
		isOpen = false
	}

	export function open() {
		isOpen = true
	}
</script>

<div
	class={$$props.class}
	use:melt={$trigger}
	aria-label="Popup button"
	on:mouseenter={() => (openOnHover ? open() : null)}
	on:mouseleave={() => (openOnHover ? close() : null)}
>
	<slot name="trigger" />
</div>

{#if isOpen && !disablePopup}
	<div
		use:melt={$content}
		transition:fade={{ duration: 100 }}
		class="w-fit rounded-md bg-surface overflow-hidden shadow-md"
		style="z-index: {zIndexes.popover}"
	>
		{#if displayArrow}
			<div use:melt={$arrow} />
		{/if}
		<slot name="content" {open} {close} />
		{#if closeButton}
			<button class="close" use:melt={$closeElement}>
				<X class="size-3" />
			</button>
		{/if}
	</div>
{/if}

<style lang="postcss">
	.close {
		@apply absolute right-1.5 top-1.5 flex h-7 w-7 items-center justify-center rounded-full;
		@apply text-primary  transition-colors hover:bg-surface-hover;
		@apply focus-visible:ring focus-visible:ring-gray-400 focus-visible:ring-offset-2;
		@apply bg-surface p-0 text-sm font-medium;
	}
</style>
