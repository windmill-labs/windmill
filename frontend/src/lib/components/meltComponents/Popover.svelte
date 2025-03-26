<script context="module" lang="ts">
	import { writable } from 'svelte/store'
	const activePopover = writable<{ id: string | null; close: (() => void) | null }>({
		id: null,
		close: null
	})
</script>

<script lang="ts">
	import { createPopover, createSync, melt } from '@melt-ui/svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import { pointerDownOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'

	export let closeButton: boolean = false
	export let displayArrow: boolean = false
	export let placement: Placement = 'bottom'
	export let disablePopup: boolean = false
	export let openOnHover: boolean = false
	export let floatingConfig: any | undefined = undefined
	export let usePointerDownOutside: boolean = false
	export let closeOnOutsideClick: boolean = true
	export let contentClasses: string = ''
	export let portal: string | HTMLElement | null = 'body'
	export let closeOnOtherPopoverOpen: boolean = false
	export let disabled: boolean = false

	const dispatch = createEventDispatcher()

	const {
		elements: { trigger, content, arrow, close: closeElement },
		states,
		options: { closeOnOutsideClick: closeOnOutsideClickOption },
		ids: { content: popoverId }
	} = createPopover({
		forceVisible: true,
		positioning: floatingConfig ?? {
			placement
		},
		portal,
		onOpenChange: ({ curr, next }) => {
			if (curr != next) {
				dispatch('openChange', next)
			}
			if (closeOnOtherPopoverOpen) {
				if (next) {
					// Close previous popover if exists
					if ($activePopover.close && $activePopover.id !== $popoverId) {
						$activePopover.close()
					}
					// Set this popover as active
					activePopover.set({ id: $popoverId, close })
				} else if ($activePopover.id === $popoverId) {
					activePopover.set({ id: null, close: null })
				}
			}
			return next
		}
	})

	export let isOpen = false
	const sync = createSync(states)
	$: sync.open(isOpen, (v) => (isOpen = v))

	// Allow for dynamic closeOnOutsideClick
	$: $closeOnOutsideClickOption = closeOnOutsideClick

	export function close() {
		isOpen = false
	}

	export function open() {
		isOpen = true
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-popover]')) as HTMLElement[]
	}
</script>

<button
	class={$$props.class}
	use:melt={$trigger}
	aria-label="Popup button"
	disabled={disabled}
	on:mouseenter={() => (openOnHover ? open() : null)}
	on:mouseleave={() => (openOnHover ? close() : null)}
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: false,
		exclude: getMenuElements
	}}
	on:pointerdown_outside={() => {
		if (usePointerDownOutside) {
			close()
		}
	}}
	data-popover
	on:click
>
	<slot name="trigger" {isOpen} />
</button>

{#if isOpen && !disablePopup}
	<div
		use:melt={$content}
		transition:fade={{ duration: 0 }}
		class={twMerge(' w-fit border rounded-md bg-surface shadow-lg', contentClasses, `z-[5001]`)}
		data-popover
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
