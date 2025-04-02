<script context="module" lang="ts">
	import { writable } from 'svelte/store'
	const activeDropdown = writable<{ id: string | null; close: (() => void) | null }>({
		id: null,
		close: null
	})

	/**
	 * Creates a debounced function that delays invoking func until after wait milliseconds
	 * have elapsed since the last time the debounced function was invoked.
	 */
	function debounce<T extends (...args: any[]) => any>(
		func: T,
		wait: number
	): (...args: Parameters<T>) => void {
		let timeoutId: ReturnType<typeof setTimeout> | null = null

		return function (...args: Parameters<T>): void {
			if (timeoutId !== null) {
				clearTimeout(timeoutId)
			}

			timeoutId = setTimeout(() => {
				func(...args)
				timeoutId = null
			}, wait)
		}
	}
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
	import { createEventDispatcher, onDestroy } from 'svelte'

	export let items: Item[] | (() => Item[]) | (() => Promise<Item[]>) = []
	export let disabled = false
	export let placement: Placement = 'bottom-end'
	export let usePointerDownOutside = false
	export let closeOnOtherDropdownOpen = true
	export let fixedHeight = true
	export let openOnHover = false
	export let hoverDelay = 200 // Delay for hover effects in milliseconds

	const dispatch = createEventDispatcher()

	const {
		elements: { menu, item, trigger },
		states,
		ids: { menu: dropdownId }
	} = createDropdownMenu({
		positioning: {
			placement
		},
		loop: true,
		onOpenChange: ({ curr, next }) => {
			if (curr !== next) {
				openByHover = false
				dispatch('openChange', next)
			}
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
		},
		forceVisible: true
	})

	let open = false
	let hovering = false
	let openByHover = false
	let hoverTimer: ReturnType<typeof setTimeout> | null = null
	let leaveTimer: ReturnType<typeof setTimeout> | null = null

	const sync = createSync(states)
	$: sync.open(open, (v) => (open = Boolean(v)))

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

	const setHovering = (value: boolean) => {
		hovering = value
	}

	const debouncedHoverStart = debounce(() => setHovering(true), hoverDelay)
	const debouncedHoverEnd = debounce(() => setHovering(false), hoverDelay)

	// Clear any pending timers on component destruction
	onDestroy(() => {
		if (hoverTimer) clearTimeout(hoverTimer)
		if (leaveTimer) clearTimeout(leaveTimer)
	})

	$: if (openOnHover && hovering && !open) {
		open = true
		openByHover = true
	} else {
		if (openByHover && !hovering) {
			open = false
			openByHover = false
		}
	}
</script>

<ResolveOpen {open} on:open on:close />

<button
	class={twMerge('w-full flex items-center justify-end', fixedHeight && 'h-8', $$props.class)}
	use:melt={$trigger}
	{disabled}
	on:click={(e) => e.stopPropagation()}
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: false,
		exclude: getMenuElements,
		customEventName: 'pointerdown_menu'
	}}
	on:pointerdown_outside={() => {
		if (usePointerDownOutside) {
			close()
		}
	}}
	on:mouseenter={() => {
		if (leaveTimer) clearTimeout(leaveTimer)
		debouncedHoverStart()
	}}
	on:mouseleave={() => {
		if (hoverTimer) clearTimeout(hoverTimer)
		debouncedHoverEnd()
	}}
	data-menu
>
	{#if $$slots.buttonReplacement}
		<slot name="buttonReplacement" />
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

{#if open}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div
		use:melt={$menu}
		data-menu
		class="z-[6000]"
		on:mouseenter={() => {
			if (leaveTimer) clearTimeout(leaveTimer)
			debouncedHoverStart()
		}}
		on:mouseleave={() => {
			if (hoverTimer) clearTimeout(hoverTimer)
			debouncedHoverEnd()
		}}
	>
		<div
			class="bg-surface border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto py-1 max-h-[50vh]"
		>
			<DropdownV2Inner items={computeItems} meltItem={item} />
		</div>
	</div>
{/if}
