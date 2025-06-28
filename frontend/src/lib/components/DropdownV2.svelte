<script module lang="ts">
	import { writable } from 'svelte/store'
	const activeDropdown = writable<{ id: string | null; close: (() => void) | null }>({
		id: null,
		close: null
	})
</script>

<script lang="ts">
	import { MoreVertical } from 'lucide-svelte'
	import type { Placement } from '@floating-ui/core'
	import type { Item } from '$lib/utils'
	import DropdownV2Inner from './DropdownV2Inner.svelte'
	import { pointerDownOutside } from '$lib/utils'
	import { createDropdownMenu, melt, createSync } from '@melt-ui/svelte'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import { triggerableByAI } from '$lib/actions/triggerableByAI'
	import { _ } from 'ag-grid-community'
	import { untrack } from 'svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		items?: Item[] | (() => Item[]) | (() => Promise<Item[]>)
		disabled?: boolean
		placement?: Placement
		usePointerDownOutside?: boolean
		closeOnOtherDropdownOpen?: boolean
		fixedHeight?: boolean
		hidePopup?: boolean
		open?: boolean
		customWidth?: number | undefined
		customMenu?: boolean
		class?: string | undefined
		buttonReplacement?: import('svelte').Snippet
		menu?: import('svelte').Snippet
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		items = [],
		disabled = false,
		placement = 'bottom-end',
		usePointerDownOutside = false,
		closeOnOtherDropdownOpen = true,
		fixedHeight = true,
		hidePopup = false,
		open = $bindable(false),
		customWidth = undefined,
		customMenu = false,
		class: classNames = undefined,
		buttonReplacement,
		menu
	}: Props = $props()

	let buttonEl: HTMLButtonElement | undefined = $state(undefined)

	const {
		elements: { menu: menuEl, item, trigger },
		states,
		ids: { menu: dropdownId }
	} = createDropdownMenu({
		positioning: {
			placement
		},
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
	class={twMerge('w-full flex items-center justify-end', fixedHeight && 'h-8', classNames)}
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
	data-menu
>
	{#if buttonReplacement}
		{@render buttonReplacement?.()}
	{:else}
		<Button
			nonCaptureEvent
			size="xs"
			color="light"
			startIcon={{ icon: MoreVertical }}
			btnClasses="bg-transparent"
		/>
	{/if}
</button>

{#if open && !hidePopup}
	<div use:melt={$menuEl} data-menu class="z-[6000] transition-all duration-100">
		{#if customMenu}
			{@render menu?.()}
		{:else}
			<div
				class="bg-surface border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto py-1 max-h-[50vh]"
				style={customWidth ? `width: ${customWidth}px` : ''}
			>
				<DropdownV2Inner {aiId} items={computeItems} meltItem={item} />
			</div>
		{/if}
	</div>
{/if}
