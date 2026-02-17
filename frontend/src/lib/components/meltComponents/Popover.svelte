<script context="module" lang="ts">
	import { writable } from 'svelte/store'
	const activePopover = writable<{ id: string | null; close: (() => void) | null }>({
		id: null,
		close: null
	})
</script>

<script lang="ts">
	import { createPopover, createSync, melt } from '@melt-ui/svelte'
	import { fly } from 'svelte/transition'
	import { X, Minimize2, Maximize2 } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import { debounce, pointerDownOutside } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import { Button } from '$lib/components/common'
	import DocLink from '$lib/components/apps/editor/settingsPanel/DocLink.svelte'
	import type { FloatingConfig } from '@melt-ui/svelte/internal/actions/floating'
	import type { EscapeBehaviorType } from '@melt-ui/svelte/internal/actions'

	export let closeButton: boolean = false
	export let displayArrow: boolean = false
	export let placement: Placement = 'bottom'
	export let disablePopup: boolean = false
	export let openOnHover: boolean = false
	export let debounceDelay: number = 0
	export let floatingConfig: any | undefined = undefined
	export let usePointerDownOutside: boolean = false
	export let closeOnOutsideClick: boolean = true
	export let contentClasses: string = ''
	export let contentStyle: string = ''
	export let portal: string | HTMLElement | null = 'body'
	export let closeOnOtherPopoverOpen: boolean = false
	export let allowFullScreen: boolean = false
	export let extraProps: Record<string, any> = {}
	export let disabled: boolean = false
	export let documentationLink: string | undefined = undefined
	export let disableFocusTrap: boolean = false
	export let escapeBehavior: EscapeBehaviorType = 'close'
	export let enableFlyTransition: boolean = false
	export let onKeyDown: (e: KeyboardEvent) => void = () => {}
	export let onClose: () => void = () => {}
	/**
	 * If provided, the popover will only open if the click is on the element with the given id.
	 */
	export let targetId: string | undefined = undefined
	/**
	 * Additional CSS selectors whose matching elements should be excluded from outside-click detection.
	 */
	export let excludeSelectors: string | undefined = undefined

	let fullScreen = false
	const dispatch = createEventDispatcher()

	// Dynamic portal: use 'body' when fullscreen, otherwise use the provided portal
	$: dynamicPortal = fullScreen ? 'body' : portal

	function clearTimers() {
		clearDebounceClose()
	}

	// Cleanup timers on component destruction
	import { onDestroy } from 'svelte'
	import type { MeltEventHandler } from '@melt-ui/svelte/internal/types'
	onDestroy(clearTimers)

	const {
		elements: { trigger, content, arrow, close: closeElement, overlay },
		states,
		options: { closeOnOutsideClick: closeOnOutsideClickOption, positioning, portal: portalOption },
		ids: { content: popoverId }
	} = createPopover({
		forceVisible: true,
		portal: dynamicPortal,
		disableFocusTrap,
		escapeBehavior,
		onOpenChange: ({ curr, next }) => {
			if (curr != next) {
				dispatch('openChange', next)
				if (!next) {
					onClose()
				}
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

	$positioning = floatingConfig ?? {
		placement,
		strategy: 'absolute',
		gutter: 8,
		overflowPadding: 16,
		flip: true,
		fitViewport: true,
		overlap: false
	}

	// Update portal reactively when fullscreen state changes
	$: if (portalOption) {
		$portalOption = dynamicPortal
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

	export function updatePositioning(pos: FloatingConfig) {
		if (positioning) {
			$positioning = pos
		}
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		const selector = excludeSelectors
			? `[data-popover], ${excludeSelectors}`
			: '[data-popover]'
		return Array.from(document.querySelectorAll(selector)) as HTMLElement[]
	}

	let { debounced: debounceClose, clearDebounce: clearDebounceClose } = debounce(
		() => openOnHover && close(),
		debounceDelay
	)

	const handleClick: MeltEventHandler<PointerEvent> = (event) => {
		if (targetId) {
			const target = event.detail.originalEvent.target as Element
			const targetElement = target.closest(`#${targetId}`)
			if (!targetElement) {
				event.preventDefault()
				event.stopPropagation()
				return
			}
		}
	}
</script>

<svelte:window onkeydown={(e) => isOpen && onKeyDown(e)} />

<button
	class={$$props.class}
	use:melt={$trigger}
	aria-label="Popup button"
	disabled={disablePopup || disabled}
	on:mouseenter={() => {
		if (openOnHover) {
			open()
			clearDebounceClose()
		}
	}}
	on:mouseleave={debounceClose}
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
	on:m-click={handleClick}
	on:click
>
	<slot name="trigger" {isOpen} />
</button>

{#if fullScreen && isOpen && !disablePopup}
	<div use:melt={$overlay} class="fixed inset-0 z-10 bg-black/50"></div>
{/if}

{#if isOpen && !disablePopup}
	<div
		on:mouseenter={() => {
			if (openOnHover) {
				open()
				clearDebounceClose()
			}
		}}
		on:mouseleave={debounceClose}
		use:melt={$content}
		transition:fly={{ duration: enableFlyTransition ? 100 : 0, y: -16 }}
		class={twMerge(
			'relative dark:border rounded-md bg-surface-tertiary shadow-lg',
			fullScreen
				? `fixed !top-1/2 !left-1/2 !-translate-x-1/2 !-translate-y-1/2 !resize-none`
				: 'w-fit',
			contentClasses,
			`z-[5001]`
		)}
		data-popover
		{...extraProps}
		style={fullScreen ? `width: 90vw; max-width: 800px; height: 90vh;` : contentStyle}
	>
		{#if displayArrow}
			<div use:melt={$arrow}></div>
		{/if}
		{#if allowFullScreen}
			<div class="absolute top-0 right-0 z-10">
				<Button
					on:click={() => (fullScreen = !fullScreen)}
					variant="subtle"
					unifiedSize="sm"
					btnClasses="text-secondary"
					iconOnly
					startIcon={fullScreen ? { icon: Minimize2 } : { icon: Maximize2 }}
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
		@apply bg-surface p-0 text-sm font-semibold;
	}
</style>
