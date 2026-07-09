<script module lang="ts">
	import { writable } from 'svelte/store'
	const activeDropdown = writable<{ id: string | null; close: (() => void) | null }>({
		id: null,
		close: null
	})
</script>

<script lang="ts">
	import { EllipsisVertical } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import type { Item } from '$lib/utils'
	import DropdownV2Inner from './DropdownV2Inner.svelte'
	import { pointerDownOutside } from '$lib/utils'
	import { createDropdownMenu, melt, createSync } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { untrack } from 'svelte'
	import { placementFly } from '$lib/utils/placementFly'
	import { ButtonType } from './common/button/model'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		items?: Item[] | (() => Item[]) | (() => Promise<Item[]>)
		disabled?: boolean
		placement?: Placement
		usePointerDownOutside?: boolean
		closeOnOtherDropdownOpen?: boolean
		// When false the menu stays open after an item is selected (melt's closeOnItemClick).
		// Consumers that keep the menu open must close it themselves where appropriate.
		// Read once at menu creation (like `placement`); changing it after mount has no effect.
		closeOnItemClick?: boolean
		fixedHeight?: boolean
		hidePopup?: boolean
		open?: boolean
		customWidth?: number | undefined
		customMenu?: boolean
		class?: string | undefined
		btnId?: string | undefined
		enableFlyTransition?: boolean
		size?: ButtonType.UnifiedSize
		btnText?: string
		buttonReplacement?: import('svelte').Snippet
		// In customMenu mode the snippet receives the melt-ui `item` action store
		// (so consumers can wrap rows in <MenuItem> for arrow-key navigation + aria)
		// and `builders` (so they can compose melt submenus, e.g. via DropdownSubmenuItem).
		menu?: import('svelte').Snippet<
			[
				{
					item: MenubarMenuElements['item']
					close: () => void
					builders: ReturnType<typeof createDropdownMenu>['builders']
				}
			]
		>
		maxHeight?: string | undefined
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		items = [],
		disabled = false,
		placement = 'bottom-end',
		usePointerDownOutside = false,
		closeOnOtherDropdownOpen = true,
		closeOnItemClick = true,
		fixedHeight = true,
		hidePopup = false,
		open = $bindable(false),
		customWidth = undefined,
		customMenu = false,
		class: classNames = undefined,
		enableFlyTransition = false,
		size = 'md',
		btnText = '',
		btnId = undefined,
		buttonReplacement,
		menu,
		maxHeight = undefined
	}: Props = $props()

	let buttonEl: HTMLButtonElement | undefined = $state(undefined)

	const {
		elements: { menu: menuEl, item, trigger },
		builders,
		states,
		ids: { menu: dropdownId }
	} = createDropdownMenu({
		positioning: {
			placement: untrack(() => placement)
		},
		closeOnItemClick: untrack(() => closeOnItemClick),
		loop: true,
		onOpenChange: ({ next }) => {
			if (closeOnOtherDropdownOpen) {
				if (next) {
					// Close previous dropdown if exists
					if ($activeDropdown.close && $activeDropdown.id !== $dropdownId) {
						$activeDropdown.close()
					}
					// Set this dropdown as active
					activeDropdown.set({ id: $dropdownId, close })
				} else if ($activeDropdown.id === $dropdownId) {
					activeDropdown.set({ id: null, close: null })
				}
			}
			return next
		}
	})

	const sync = createSync(states)
	$effect(() => {
		open
		untrack(() => {
			sync.open(open, (v) => (open = Boolean(v)))
		})
	})

	export function close() {
		open = false
	}

	async function computeItems(): Promise<Item[]> {
		if (typeof items === 'function') {
			const result = await items()
			return Array.isArray(result) ? result.filter((item) => !item.hide) : []
		} else {
			return items.filter((item) => !item.hide)
		}
	}
	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-menu]')) as HTMLElement[]
	}
</script>

<ResolveOpen {open} on:open on:close />

<button
	bind:this={buttonEl}
	use:triggerableByAI={{
		id: aiId,
		description: aiDescription,
		callback: () => buttonEl?.click()
	}}
	class={twMerge('flex items-center justify-end', fixedHeight && 'h-8', classNames)}
	use:melt={$trigger}
	{disabled}
	onclick={(e) => e.stopPropagation()}
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
	id={btnId}
	data-menu
>
	{#if buttonReplacement}
		{@render buttonReplacement?.()}
	{:else}
		<Button
			nonCaptureEvent
			unifiedSize={size}
			variant="subtle"
			startIcon={{ icon: EllipsisVertical }}
			btnClasses="bg-transparent"
			iconOnly={!btnText}
		>
			{btnText}
		</Button>
	{/if}
</button>

{#if open && !hidePopup}
	<div
		use:melt={$menuEl}
		data-menu
		class="z-[6000] transition-all duration-100"
		transition:placementFly={{ duration: enableFlyTransition ? 100 : 0, placement }}
	>
		{#if customMenu}
			{@render menu?.({ item, close, builders })}
		{:else}
			<div
				class="bg-surface-tertiary dark:border w-56 origin-top-right rounded-lg shadow-lg focus:outline-none overflow-y-auto py-1"
				style={`${customWidth ? `width: ${customWidth}px;` : ''} max-height: ${maxHeight || '50vh'};`}
			>
				<DropdownV2Inner {aiId} items={computeItems} meltItem={item} {builders} />
			</div>
		{/if}
	</div>
{/if}
