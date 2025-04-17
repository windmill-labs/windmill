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
	import { X, Minimize2, Maximize2 } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import { pointerDownOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import { Button } from '$lib/components/common'
	import DocLink from '$lib/components/apps/editor/settingsPanel/DocLink.svelte'

	export let closeButton: boolean = false
	export let displayArrow: boolean = false
	export let placement: Placement = 'bottom'
	export let disablePopup: boolean = false
	export let openOnHover: boolean = false
	export let floatingConfig: any | undefined = undefined
	export let usePointerDownOutside: boolean = false
	export let closeOnOutsideClick: boolean = true
	export let contentClasses: string = ''
	export let contentStyle: string = ''
	export let portal: string | HTMLElement | null = 'body'
	export let closeOnOtherPopoverOpen: boolean = false
	export let allowFullScreen: boolean = false
	export let fullScreenWidthOffset: number = 0
	export let extraProps: Record<string, any> = {}
	export let disabled: boolean = false
	export let documentationLink: string | undefined = undefined

	let fullScreen = false

	const dispatch = createEventDispatcher()

	const {
		elements: { trigger, content, arrow, close: closeElement, overlay },
		states,
		options: { closeOnOutsideClick: closeOnOutsideClickOption, positioning },
		ids: { content: popoverId }
	} = createPopover({
		forceVisible: true,
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
			if (fullScreen && !next) {
				fullScreen = false
			}
			return next
		}
	})

	$: $positioning = floatingConfig ?? {
		placement,
		strategy: fullScreen ? 'fixed' : 'absolute',
		x: fullScreen ? 0 : undefined,
		y: fullScreen ? 0 : undefined
	}

	export let isOpen = false
	const sync = createSync(states)
	$: sync.open(isOpen, (v) => (isOpen = v))

	// Allow for dynamic closeOnOutsideClick
	$: $closeOnOutsideClickOption = usePointerDownOutside ? false : closeOnOutsideClick

	export function close() {
		isOpen = false
	}

	export function open() {
		isOpen = true
	}

	export function isOpened() {
		return isOpen
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-popover]')) as HTMLElement[]
	}
</script>

<button
	class={$$props.class}
	use:melt={$trigger}
	aria-label="Popup button"
	disabled={disablePopup || disabled}
	on:mouseenter={() => (openOnHover ? open() : null)}
	on:mouseleave={() => (openOnHover ? close() : null)}
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: false,
		exclude: getMenuElements
	}}
	on:pointerdown_outside={() => {
		if (usePointerDownOutside) {
			if (fullScreen) fullScreen = false
			else close()
		}
	}}
	data-popover
	on:click
>
	<slot name="trigger" {isOpen} />
</button>

{#if fullScreen && isOpen && !disablePopup}
	<div use:melt={$overlay} class="fixed inset-0 z-10 bg-black/50"></div>
{/if}

{#if isOpen && !disablePopup}
	<div
		use:melt={$content}
		transition:fade={{ duration: 0 }}
		class={twMerge(
			'relative border rounded-md bg-surface shadow-lg',
			fullScreen
				? `fixed !top-1/2 !left-1/2 !-translate-x-1/2 !-translate-y-1/2 !resize-none`
				: 'w-fit',
			contentClasses,
			`z-[5001]`
		)}
		data-popover
		{...extraProps}
		style={fullScreen
			? `width: calc(90vw - ${fullScreenWidthOffset}px); height: 90vh;`
			: contentStyle}
	>
		{#if displayArrow}
			<div use:melt={$arrow}></div>
		{/if}
		{#if allowFullScreen}
			<div class="absolute top-0 right-0 z-10">
				<Button
					on:click={() => (fullScreen = !fullScreen)}
					color="light"
					size="xs2"
					iconOnly
					startIcon={fullScreen ? { icon: Minimize2 } : { icon: Maximize2 }}
					btnClasses="text-gray-400"
				/>
			</div>
		{/if}
		{#if documentationLink}
			<div class="absolute right-1.5 top-1.5">
				<DocLink docLink={documentationLink} size="sm" />
			</div>
		{/if}
		{#if closeButton}
			<button class="close" use:melt={$closeElement}>
				<X class="size-3" />
			</button>
		{/if}

		<slot name="content" {open} {close} />
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
