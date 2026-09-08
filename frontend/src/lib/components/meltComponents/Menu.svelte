<script lang="ts">
	import { untrack } from 'svelte'
	import { overlayPortalTarget } from '$lib/components/common/overlayHost.svelte'
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { placementFly } from '$lib/utils/placementFly'
	import { melt, createSync } from '@melt-ui/svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'
	import { debounce, pointerDownOutside } from '$lib/utils'

	import { twMerge } from 'tailwind-merge'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'
	import { watch } from 'runed'

	interface Props {
		placement?: Placement
		justifyEnd?: boolean
		lightMode?: boolean
		maxHeight?: number
		disabled?: boolean
		createMenu: MenubarBuilders['createMenu']
		invisible?: boolean
		usePointerDownOutside?: boolean
		menuClass?: string
		open?: boolean
		renderContent?: boolean
		// Move the scroll/overflow onto an inner wrapper instead of the melt element. The
		// melt element is the fixed-positioned containing block for any submenu, so overflow
		// on it clips submenus that open to the side. Opt in only when using a submenu — the
		// default keeps the existing single-element markup untouched for every other menu.
		submenuSafe?: boolean
		// Open on hover without pinning, so the menu closes again once the pointer leaves.
		// A click then pins it open until a click outside or on the trigger.
		openOnHover?: boolean
		// Grace period before a hover-opened menu closes, so the pointer can travel from
		// the trigger to the content.
		debounceDelay?: number
		classNames?: string
		triggr?: import('svelte').Snippet<[any]>
		children?: import('svelte').Snippet<[any]>
		class?: string
	}

	let {
		placement = 'right-start',
		justifyEnd = false,
		lightMode = false,
		maxHeight = 900,
		disabled = false,
		createMenu,
		invisible = false,
		usePointerDownOutside = false,
		menuClass = '',
		open = $bindable(false),
		renderContent = false,
		submenuSafe = false,
		openOnHover = false,
		debounceDelay = 150,
		class: classNames = '',
		triggr,
		children
	}: Props = $props()

	// Overlays belong to the enclosing pane when there is one — see overlayHost.
	const hostPortal = overlayPortalTarget('body')

	// Use the passed createMenu function
	const menu = untrack(() => createMenu)({
		portal: untrack(() => hostPortal()),
		positioning: {
			placement: untrack(() => placement),
			fitViewport: true,
			strategy: 'fixed'
		},
		loop: true,
		// Hover tooltips (e.g. NameIdTooltip on menu rows) portal to body, so a
		// click inside one — like its copy button — registers as an outside click.
		// Veto the close so interacting with a tooltip doesn't tear the menu down.
		onOutsideClick: (e) => {
			if ((e.target as HTMLElement)?.closest?.('[data-melt-tooltip-content]')) {
				e.preventDefault()
			}
		}
	})

	//Melt
	const {
		elements: { trigger, menu: menuElement, item },
		builders,
		states,
		options: { portal: portalOption }
	} = menu

	$effect(() => {
		$portalOption = hostPortal()
	})

	const sync = createSync(states)
	watch(
		() => open,
		() => sync.open(open, (v) => (open = Boolean(v)))
	)

	export function close() {
		open = false
	}

	// A hover-opened menu is unpinned; a click pins it until a click outside or on the trigger.
	// Handed to the triggr snippet so the trigger can show the pinned state.
	let pinned = $state(false)
	let triggerEl: HTMLElement | undefined = $state()
	let clickingTrigger = false

	watch(
		() => open,
		() => {
			if (!open) {
				pinned = false
				cancelPendingClose()
			}
		}
	)

	// Setting `open` doesn't open a menubar menu: melt shows the content only once the
	// trigger's own click handler has registered it as the active trigger. So every
	// hover-driven open and close goes through a click on the trigger instead.
	function toggleViaTrigger() {
		const el = triggerEl?.querySelector('[data-melt-menubar-trigger]')
		if (!(el instanceof HTMLElement)) return
		// click() dispatches synchronously, so the flag only covers our own event.
		clickingTrigger = true
		try {
			el.click()
		} finally {
			clickingTrigger = false
		}
	}

	const { debounced: debounceClose, clearDebounce: cancelPendingClose } = debounce(
		() => {
			if (open && !pinned) toggleViaTrigger()
		},
		untrack(() => debounceDelay)
	)

	function handleHoverEnter() {
		if (!openOnHover) return
		cancelPendingClose()
		if (!open) toggleViaTrigger()
	}

	function scheduleClose() {
		if (!openOnHover) return
		cancelPendingClose()
		if (pinned) return
		// The content is portaled away from the trigger, so moving between the two fires a
		// leave on the one being left; the delay lets the matching enter cancel it.
		debounceClose()
	}

	function handleTriggerClick(e: MouseEvent) {
		if (!openOnHover || clickingTrigger) return
		cancelPendingClose()
		if (open && !pinned) {
			// Hover already opened it: swallow the click before melt's trigger handler
			// toggles it shut, and pin it instead.
			e.preventDefault()
			e.stopPropagation()
			pinned = true
			return
		}
		// Melt handles the toggle itself: closed becomes open and pinned, pinned closes.
		pinned = !open
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		// Tooltip content counts as menu territory for the same reason as the
		// onOutsideClick veto above.
		return Array.from(
			document.querySelectorAll('[data-menu], [data-melt-tooltip-content]')
		) as HTMLElement[]
	}
</script>

<div class={twMerge('w-full h-8', classNames)}>
	<ResolveOpen {open} on:open on:close />

	<button
		bind:this={triggerEl}
		class={twMerge('w-full h-full', justifyEnd ? 'flex justify-end' : '')}
		{disabled}
		onmouseenter={handleHoverEnter}
		onmouseleave={scheduleClose}
		onclickcapture={handleTriggerClick}
		use:pointerDownOutside={{
			capture: true,
			stopPropagation: false,
			exclude: getMenuElements,
			customEventName: 'pointerdown_menu',
			onClickOutside: () => {
				if (usePointerDownOutside) {
					close()
				}
			}
		}}
		data-menu
	>
		{@render triggr?.({ trigger, pinned })}
	</button>

	<!--svelte-ignore a11y_no_static_element_interactions-->
	{#if open || renderContent}
		<div
			use:melt={$menuElement}
			data-menu
			onmouseenter={cancelPendingClose}
			onmouseleave={scheduleClose}
			transition:placementFly={{ duration: 100, placement }}
			class={twMerge(
				'z-[6000] border w-56 origin-top-right rounded-md shadow-md focus:outline-none',
				// Default: scroll on the melt element. submenuSafe moves it to the inner
				// wrapper so a side-opening submenu isn't clipped by this element's overflow.
				submenuSafe ? '' : 'overflow-y-auto',
				lightMode ? 'bg-surface-inverse' : 'bg-surface',
				invisible ? 'opacity-0' : '',
				menuClass
			)}
			onclick={bubble('click')}
		>
			<div
				class={twMerge('py-1', submenuSafe ? 'overflow-y-auto' : '')}
				style="max-height: min({maxHeight}px, calc(100vh - 6rem)); "
			>
				{@render children?.({ item, open, builders })}
			</div>
		</div>
	{/if}
</div>
