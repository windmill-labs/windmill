<script module lang="ts">
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
	import { Button } from '$lib/components/common'
	import DocLink from '$lib/components/apps/editor/settingsPanel/DocLink.svelte'
	import type { FloatingConfig } from '@melt-ui/svelte/internal/actions/floating'
	import type { EscapeBehaviorType } from '@melt-ui/svelte/internal/actions'
	import { onDestroy } from 'svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		closeButton?: boolean
		displayArrow?: boolean
		placement?: Placement
		disablePopup?: boolean
		/** When true, the trigger renders as a non-interactive span instead of a button */
		notClickable?: boolean
		openOnHover?: boolean
		debounceDelay?: number
		floatingConfig?: any | undefined
		usePointerDownOutside?: boolean
		closeOnOutsideClick?: boolean
		/** @deprecated Use `closeOnOutsideClick` instead */
		closeOnClickOutside?: boolean
		contentClasses?: string
		/** @deprecated Use `contentClasses` instead */
		containerClasses?: string
		contentStyle?: string
		portal?: string | HTMLElement | null
		closeOnOtherPopoverOpen?: boolean
		allowFullScreen?: boolean
		extraProps?: Record<string, any>
		disabled?: boolean
		documentationLink?: string | undefined
		disableFocusTrap?: boolean
		openFocus?: string | HTMLElement | (() => HTMLElement | null) | null | undefined
		escapeBehavior?: EscapeBehaviorType
		enableFlyTransition?: boolean
		onKeyDown?: (e: KeyboardEvent) => void
		onClose?: () => void
		/**
		 * If provided, the popover will only open if the click is on the element with the given id.
		 */
		targetId?: string | undefined
		/**
		 * Additional CSS selectors whose matching elements should be excluded from outside-click detection.
		 */
		excludeSelectors?: string | undefined
		isOpen?: boolean
		class?: string
		onclick?: (e: MouseEvent) => void
		onOpenChange?: (isOpen: boolean) => void
		trigger?: Snippet<[{ isOpen: boolean }]>
		content?: Snippet<[{ open: () => void; close: () => void }]>
	}

	let {
		closeButton = false,
		displayArrow = false,
		placement = 'bottom',
		disablePopup = false,
		notClickable = false,
		openOnHover = false,
		debounceDelay = 0,
		floatingConfig = undefined,
		usePointerDownOutside = false,
		closeOnOutsideClick: closeOnOutsideClickProp = true,
		closeOnClickOutside: closeOnClickOutsideProp = undefined,
		contentClasses: contentClassesProp = '',
		containerClasses: containerClassesProp = undefined,
		contentStyle = '',
		portal = 'body',
		closeOnOtherPopoverOpen = false,
		allowFullScreen = false,
		extraProps = {},
		disabled = false,
		documentationLink = undefined,
		disableFocusTrap = false,
		openFocus = undefined,
		escapeBehavior = 'close',
		enableFlyTransition = false,
		onKeyDown: onKeyDownProp = () => {},
		onClose: onCloseProp = () => {},
		targetId = undefined,
		excludeSelectors = undefined,
		isOpen = $bindable(false),
		class: className = undefined,
		onclick: onclickProp = undefined,
		onOpenChange: onOpenChangeProp = undefined,
		trigger: triggerSnippet,
		content: contentSnippet
	}: Props = $props()

	// Alias props: containerClasses -> contentClasses, closeOnClickOutside -> closeOnOutsideClick
	const contentClasses = $derived(containerClassesProp ?? contentClassesProp)
	const closeOnOutsideClick = $derived(closeOnClickOutsideProp ?? closeOnOutsideClickProp)

	let fullScreen = $state(false)

	// Dynamic portal: use 'body' when fullscreen, otherwise use the provided portal
	let dynamicPortal = $derived(fullScreen ? 'body' : portal)

	function clearTimers() {
		clearDebounceClose()
	}

	// Cleanup timers on component destruction
	onDestroy(clearTimers)

	const {
		elements: { trigger: triggerEl, content: contentEl, arrow, close: closeElement, overlay },
		states,
		options: { closeOnOutsideClick: closeOnOutsideClickOption, positioning, portal: portalOption },
		ids: { content: popoverId }
	} = createPopover({
		forceVisible: true,
		portal: portal,
		disableFocusTrap,
		escapeBehavior,
		openFocus,
		onOpenChange: ({ curr, next }) => {
			if (curr != next) {
				onOpenChangeProp?.(next)
				if (!next) {
					onCloseProp()
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
	$effect(() => {
		if (portalOption) {
			$portalOption = dynamicPortal
		}
	})

	const sync = createSync(states)
	$effect(() => {
		sync.open(isOpen, (v) => (isOpen = v))
	})

	// Allow for dynamic closeOnOutsideClick
	$effect(() => {
		$closeOnOutsideClickOption = usePointerDownOutside ? false : closeOnOutsideClick
	})

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

	const handleClick = (event: MouseEvent) => {
		if (targetId) {
			const target = event.target as Element
			const targetElement = target.closest(`#${targetId}`)
			if (!targetElement) {
				event.preventDefault()
				event.stopPropagation()
				return
			}
		}
	}
</script>

<svelte:window onkeydown={(e) => isOpen && onKeyDownProp(e)} />

{#if notClickable}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<span
		class={className}
		use:melt={$triggerEl}
		onmouseenter={() => {
			if (openOnHover) {
				open()
				clearDebounceClose()
			}
		}}
		onmouseleave={debounceClose}
		data-popover
	>
		{@render triggerSnippet?.({ isOpen })}
	</span>
{:else}
	<button
		class={className}
		use:melt={$triggerEl}
		aria-label="Popup button"
		disabled={disablePopup || disabled}
		onmouseenter={() => {
			if (openOnHover) {
				open()
				clearDebounceClose()
			}
		}}
		onmouseleave={debounceClose}
		use:pointerDownOutside={{
			capture: true,
			stopPropagation: false,
			exclude: getMenuElements,
			onClickOutside: () => {
				if (usePointerDownOutside) {
					if (fullScreen) fullScreen = false
					else close()
				}
			}
		}}
		data-popover
		onclick={(e) => {
			handleClick(e)
			onclickProp?.(e)
		}}
	>
		{@render triggerSnippet?.({ isOpen })}
	</button>
{/if}

{#if fullScreen && isOpen && !disablePopup}
	<div use:melt={$overlay} class="fixed inset-0 z-10 bg-black/50"></div>
{/if}

{#if isOpen && !disablePopup}
	<div
		onmouseenter={() => {
			if (openOnHover) {
				open()
				clearDebounceClose()
			}
		}}
		onmouseleave={debounceClose}
		use:melt={$contentEl}
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
					onclick={() => (fullScreen = !fullScreen)}
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

		{@render contentSnippet?.({ open, close })}
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
