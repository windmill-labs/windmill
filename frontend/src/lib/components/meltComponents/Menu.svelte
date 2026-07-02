<script lang="ts">
	import { untrack } from 'svelte'
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { placementFly } from '$lib/utils/placementFly'
	import { melt, createSync } from '@melt-ui/svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'
	import { pointerDownOutside } from '$lib/utils'

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
		class: classNames = '',
		triggr,
		children
	}: Props = $props()

	// Use the passed createMenu function
	const menu = untrack(() => createMenu)({
		positioning: {
			placement: untrack(() => placement),
			fitViewport: true,
			strategy: 'fixed'
		},
		loop: true
	})

	//Melt
	const {
		elements: { trigger, menu: menuElement, item },
		builders,
		states
	} = menu

	const sync = createSync(states)
	watch(
		() => open,
		() => sync.open(open, (v) => (open = Boolean(v)))
	)

	export function close() {
		open = false
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-menu]')) as HTMLElement[]
	}
</script>

<div class={twMerge('w-full h-8', classNames)}>
	<ResolveOpen {open} on:open on:close />

	<button
		class={twMerge('w-full h-full', justifyEnd ? 'flex justify-end' : '')}
		{disabled}
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
		{@render triggr?.({ trigger })}
	</button>

	<!--svelte-ignore a11y_no_static_element_interactions-->
	{#if open || renderContent}
		<div
			use:melt={$menuElement}
			data-menu
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
