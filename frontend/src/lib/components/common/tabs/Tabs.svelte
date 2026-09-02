<script lang="ts">
	import { setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import ScrollableX from '../ScrollableX.svelte'
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'

	const dispatch = createEventDispatcher<{ selected: string }>()

	interface Props {
		selected: string
		hideTabs?: boolean
		class?: string
		wrapperClass?: string
		style?: string
		hashNavigation?: boolean
		values?: string[] | undefined
		children?: import('svelte').Snippet<[any]>
		content?: import('svelte').Snippet
		/**
		 * If true, the tab component will only update the internal store when a tab is clicked,
		 * but will NOT immediately update the bindable 'selected' prop. This allows the parent
		 * component to control when the tab actually changes (e.g., after navigation completes).
		 * Use this when you want to prevent navigation before checking for unsaved changes.
		 */
		deferSelectedUpdate?: boolean
		/**
		 * Draw the selection as one bar that slides between tabs, rather than a border each tab
		 * turns on. Opt-in, so every existing strip keeps the border it has: a strip whose tabs
		 * appear as their content does needs the move to be visible, and a border cannot travel.
		 * Tabs keep their own bottom border unless the caller turns it off.
		 */
		slidingIndicator?: boolean
		/** Colour of that bar. */
		indicatorClass?: string
	}

	let {
		selected = $bindable(),
		hideTabs = false,
		class: c = '',
		wrapperClass = '',
		style = '',
		hashNavigation = false,
		values = undefined,
		children,
		content,
		deferSelectedUpdate = false,
		slidingIndicator = false,
		indicatorClass = 'bg-border-normal'
	}: Props = $props()

	// Single source of truth for tab state
	const selectedStore = writable(selected)

	function update(value: string) {
		if (!deferSelectedUpdate) {
			selected = value
		}
		dispatch('selected', value)
	}

	setContext<TabsContext>('Tabs', {
		selected: selectedStore,
		update,
		hashNavigation: untrack(() => hashNavigation)
	})

	// Sync external prop changes to store (single direction: prop → store)
	$effect(() => {
		selectedStore.set(selected)
	})

	// Measured off the selected Tab rather than tracked in state: a Tab decides on its own
	// whether it is selected (prefix and otherValues matching), and its width is whatever its
	// label renders to. Zero width means nothing is selected yet, and the bar stays hidden.
	let row: HTMLDivElement | undefined = $state()
	let bar = $state({ x: 0, w: 0 })

	function measureBar() {
		const el = row?.querySelector<HTMLElement>('[data-tab-selected="true"]')
		bar = el ? { x: el.offsetLeft, w: el.offsetWidth } : { x: bar.x, w: 0 }
	}

	// Placing the bar for the first paint. Every later move comes from the observers below: a
	// Tab marks itself selected in its own update, which has not run when an effect here does,
	// so measuring from this side alone lands the bar on the tab that was selected before.
	$effect(() => {
		if (!slidingIndicator) return
		void row
		measureBar()
	})

	$effect(() => {
		if (!slidingIndicator || !row) return
		const ro = new ResizeObserver(measureBar)
		ro.observe(row)
		// The mark moving is the selection changing; a tab added or removed, or one that grows a
		// count as its content arrives, changes what the bar has to sit on.
		const mo = new MutationObserver(measureBar)
		mo.observe(row, {
			childList: true,
			subtree: true,
			attributes: true,
			attributeFilter: ['data-tab-selected']
		})
		return () => {
			ro.disconnect()
			mo.disconnect()
		}
	})

	let hashValues = $derived(values ? values.map((x) => '#' + x) : undefined)

	function hashChange() {
		if (hashNavigation) {
			const hash = window.location.hash
			if (hash && hashValues?.includes(hash)) {
				const id = hash.replace('#', '')
				update(id)
			}
		}
	}
</script>

<svelte:window onhashchange={hashChange} />
{#if !hideTabs}
	<ScrollableX class={wrapperClass}>
		<!-- `scrollbar-hidden` is inert on this non-scrolling row (ScrollableX owns the
			 scroll), but TroubleshootFlowTutorial targets it as a selector hook — keep it. -->
		<div
			bind:this={row}
			class={twMerge(
				'border-b flex flex-row whitespace-nowrap scrollbar-hidden',
				slidingIndicator ? 'relative' : '',
				c
			)}
			{style}
		>
			{@render children?.({ selected })}
			{#if slidingIndicator}
				<span
					aria-hidden="true"
					class={twMerge(
						'pointer-events-none absolute -bottom-px h-0.5 rounded-t-sm transition-[transform,width,opacity] duration-200 ease-out motion-reduce:transition-none',
						indicatorClass
					)}
					style={`left:0; width:${bar.w}px; transform:translateX(${bar.x}px); opacity:${bar.w ? 1 : 0}`}
				></span>
			{/if}
		</div>
	</ScrollableX>
{/if}
{@render content?.()}
