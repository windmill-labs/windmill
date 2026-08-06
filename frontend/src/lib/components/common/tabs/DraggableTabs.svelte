<script lang="ts" module>
	export type TabItem = {
		/** Stable identifier; used as the `[key]` for dnd and the activeId equality check. */
		id: string
		label: string
		/** Hover title. Labels truncate at 180px, so set this whenever the label can
		 * be long enough that two tabs would truncate to the same visible text. */
		title?: string
		/** Optional lucide-svelte (or compatible) component rendered at 12px before the label. */
		icon?: any
		/** Optional class applied to the icon (e.g. `text-accent` to tint it). */
		iconClass?: string
		/** Optional class applied to the label text (e.g. `text-accent` to tint it). */
		labelClass?: string
		/** Defaults to true. Set false to hide the × close button. */
		closable?: boolean
		/** Pinned tabs are rendered outside the drag zone — 'left' or 'right' of the draggable group. */
		pinned?: 'left' | 'right'
	}

	// Per-instance dnd zone `type` so sibling bars (mirrored single-view) don't
	// share svelte-dnd-action's item pool — otherwise a drag in one ghosts the
	// matching tab in the other.
	let dndZoneSeq = 0
</script>

<script lang="ts">
	import { dndzone, type DndEvent } from '@windmill-labs/svelte-dnd-action'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { untrack } from 'svelte'

	interface Props {
		tabs: TabItem[]
		activeId: string
		onSelect: (id: string) => void
		onClose?: (id: string) => void
		onReorder?: (newOrder: TabItem[]) => void
		/** Called instead of `onSelect` when the already-active tab is clicked or
		 * activated via Enter/Space — lets the active tab host a secondary affordance
		 * (e.g. toggling the breadcrumb picker rendered in `tabAccessory`). */
		onActiveClick?: (id: string) => void
		/** Extra classes for the outer tab strip. It is 32px tall unless `trailing`
		 * content is taller; a fixed height here overrides that, and going taller
		 * pulls the tabs away from the scroll bar, which stays on the bottom edge. */
		class?: string
		/** Render inside the scroll row, right after the last tab (e.g. a "+" new-tab
		 * button) — scrolls with the tabs, unlike `trailing`. */
		afterTabs?: import('svelte').Snippet
		/** Render after the right-pinned tabs, outside the scroll area so it stays
		 * pinned (e.g. a "Split with Preview" toggle). */
		trailing?: import('svelte').Snippet
		/** Render inside each tab (after the label; receives the tab + isActive). Clicks
		 * bubble to the tab unless the accessory stops them. The tab is position:relative
		 * so an `absolute inset-0 pointer-events-none` child can anchor a whole-tab
		 * popover — a *clickable* overlay would break dnd (no drags from nested buttons). */
		tabAccessory?: import('svelte').Snippet<[TabItem, boolean]>
	}

	let {
		tabs,
		activeId,
		onSelect,
		onClose,
		onReorder,
		onActiveClick,
		class: c = '',
		afterTabs,
		trailing,
		tabAccessory
	}: Props = $props()

	const pinnedLeft = $derived(tabs.filter((t) => t.pinned === 'left'))
	const middle = $derived(tabs.filter((t) => !t.pinned))
	const pinnedRight = $derived(tabs.filter((t) => t.pinned === 'right'))

	// Unique dnd zone type for this instance (see note in the module block).
	const dndType = `draggable-tabs-${dndZoneSeq++}`

	// Local list the dnd zone owns. `consider` updates only this (mid-drag it
	// holds svelte-dnd-action's shadow placeholder); we commit to the parent on
	// `finalize` so the placeholder never leaks into a sibling bar.
	let stripEl = $state<HTMLElement | undefined>(undefined)
	let dndMiddle = $state<TabItem[]>(untrack(() => middle))
	let isDragging = false
	$effect(() => {
		const next = middle
		// Re-sync from props except mid-drag, where the dnd zone owns the list.
		if (!isDragging) dndMiddle = next
	})

	// Scroll bar. The native one is hidden (`no-scrollbar`) and redrawn here: its
	// height is a WebKit-only setting, so Firefox spends 11px of the strip on a
	// bar there is no room for and clips the tabs. Ours is 4px in every engine and
	// costs no layout height at all.
	const MIN_THUMB = 24
	let scrollEl = $state<HTMLElement | undefined>(undefined)
	let scrollLeft = $state(0)
	let viewport = $state(0)
	let content = $state(0)
	const scrollable = $derived(Math.max(0, content - viewport))
	const overflowing = $derived(scrollable > 1)
	// Clamped to the viewport: a pane dragged shut leaves a few pixels of strip,
	// and an unclamped minimum-width thumb would hang out of it.
	const thumbWidth = $derived(
		overflowing ? Math.min(viewport, Math.max(MIN_THUMB, (viewport / content) * viewport)) : 0
	)
	// Clamped at both ends: `scrollLeft` is fractional on HiDPI while the widths
	// are rounded, so the ratio can tip past 1, and WebKit's elastic overscroll
	// drives it negative — either way the thumb would leave the track.
	const thumbLeft = $derived(
		scrollable > 0
			? Math.max(
					0,
					Math.min(viewport - thumbWidth, (scrollLeft / scrollable) * (viewport - thumbWidth))
				)
			: 0
	)

	function measure() {
		const el = scrollEl
		if (!el) return
		scrollLeft = el.scrollLeft
		viewport = el.clientWidth
		content = el.scrollWidth
	}

	// Both ends move independently: the viewport on a pane resize, the content as
	// tabs open, close and get renamed.
	$effect(() => {
		const el = scrollEl
		if (!el) return
		measure()
		const ro = new ResizeObserver(measure)
		ro.observe(el)
		if (el.firstElementChild) ro.observe(el.firstElementChild)
		return () => ro.disconnect()
	})

	// Drag the thumb: pointer capture keeps the gesture alive past the strip's
	// edges, and the ratio maps thumb travel back onto scroll travel. Recomputing
	// from the anchor each move (rather than accumulating) means clamping at
	// either end doesn't drift, and reading the travel live keeps a tab opening
	// mid-drag from scaling every later move against a stale track.
	function handleThumbPointerDown(e: PointerEvent) {
		const el = scrollEl
		// Primary button only: a right-click would open the context menu without
		// delivering the pointerup that ends the drag.
		if (!el || e.button !== 0) return
		e.preventDefault()
		const target = e.currentTarget as HTMLElement
		const startX = e.clientX
		const startScroll = el.scrollLeft
		target.setPointerCapture(e.pointerId)
		const onMove = (ev: PointerEvent) => {
			const travel = viewport - thumbWidth
			if (travel <= 0) return
			el.scrollLeft = startScroll + ((ev.clientX - startX) / travel) * scrollable
		}
		const onUp = (ev: PointerEvent) => {
			target.releasePointerCapture(ev.pointerId)
			target.removeEventListener('pointermove', onMove)
			target.removeEventListener('pointerup', onUp)
			target.removeEventListener('pointercancel', onUp)
		}
		target.addEventListener('pointermove', onMove)
		target.addEventListener('pointerup', onUp)
		target.addEventListener('pointercancel', onUp)
	}

	function handleConsider(e: CustomEvent<DndEvent<TabItem>>) {
		isDragging = true
		dndMiddle = e.detail.items
	}
	function handleFinalize(e: CustomEvent<DndEvent<TabItem>>) {
		isDragging = false
		dndMiddle = e.detail.items
		onReorder?.([...pinnedLeft, ...e.detail.items, ...pinnedRight])
	}

	function tabClasses(isActive: boolean) {
		return twMerge(
			'group relative inline-flex items-center gap-1.5 px-2.5 h-6 text-xs rounded-md select-none cursor-pointer whitespace-nowrap transition-colors focus:outline-none focus-visible:ring-1 focus-visible:ring-border-selected focus-visible:ring-inset',
			isActive
				? 'bg-surface-tertiary text-emphasis'
				: 'bg-transparent text-hint hover:text-secondary'
		)
	}

	function activate(tab: TabItem) {
		if (tab.id === activeId && onActiveClick) onActiveClick(tab.id)
		else onSelect(tab.id)
	}

	// Runs as a DIRECT capture listener: svelte-dnd-action's item-wrapper handler
	// swallows Enter/Space (keyboard drag) before Svelte's root-delegated keydown
	// would fire, so the tab must claim its keys first via stopPropagation.
	function handleKeydown(e: KeyboardEvent, tab: TabItem) {
		if (e.target !== e.currentTarget) return // let nested controls (close ×) act
		if (e.key === 'Delete' || e.key === 'Backspace') {
			if (tab.closable !== false) {
				e.preventDefault()
				e.stopPropagation()
				onClose?.(tab.id)
			}
		} else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
			const idx = tabs.findIndex((t) => t.id === tab.id)
			const next = e.key === 'ArrowLeft' ? idx - 1 : idx + 1
			if (next >= 0 && next < tabs.length) {
				e.preventDefault()
				e.stopPropagation()
				onSelect(tabs[next].id)
				// Selection moved — follow with focus, else the next arrow press
				// recomputes from this (stale) tab and Delete closes the wrong one.
				stripEl?.querySelector<HTMLElement>(`[data-tab-id="${CSS.escape(tabs[next].id)}"]`)?.focus()
			}
		} else if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			e.stopPropagation()
			activate(tab)
		}
	}

	function handleAuxClick(e: MouseEvent, tab: TabItem) {
		if (e.button === 1 && tab.closable !== false) {
			e.preventDefault()
			onClose?.(tab.id)
		}
	}
</script>

{#snippet tabButton(tab: TabItem)}
	{@const isActive = tab.id === activeId}
	{@const Icon = tab.icon}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		role="tab"
		data-tab-id={tab.id}
		title={tab.title}
		aria-selected={isActive}
		tabindex={isActive ? 0 : -1}
		class={twMerge(tabClasses(isActive), tab.closable !== false && 'pr-1')}
		onclick={() => activate(tab)}
		onauxclick={(e) => handleAuxClick(e, tab)}
		onkeydowncapture={(e) => handleKeydown(e, tab)}
	>
		{#if Icon}
			<Icon size={12} class={tab.iconClass} />
		{/if}
		<span class={twMerge('truncate max-w-[180px]', tab.labelClass)}>{tab.label}</span>
		{#if tabAccessory}
			<span class="inline-flex items-center">
				{@render tabAccessory(tab, isActive)}
			</span>
		{/if}
		{#if tab.closable !== false}
			<button
				type="button"
				class="opacity-0 group-hover:opacity-100 focus:opacity-100 rounded hover:bg-surface-hover w-4 h-4 inline-flex items-center justify-center"
				aria-label={`Close ${tab.label}`}
				onclick={(e) => {
					e.stopPropagation()
					onClose?.(tab.id)
				}}
			>
				<X size={10} />
			</button>
		{/if}
	</div>
{/snippet}

<div bind:this={stripEl} class={twMerge('flex items-center bg-surface min-h-8', c)}>
	<!-- The tabs centre in the full strip and the bar overlays the air under them,
	     flush with the strip's bottom edge — it takes no height of its own, so the
	     strip never resizes and the tabs sit at the same place whether or not they
	     overflow. -->
	<div class="group/scroll relative flex-1 min-w-0 self-stretch">
		<div
			bind:this={scrollEl}
			onscroll={() => scrollEl && (scrollLeft = scrollEl.scrollLeft)}
			class="h-full overflow-x-auto overflow-y-hidden no-scrollbar pl-1"
		>
			<!-- `w-max`: without it the row is pinned to the viewport width and the tabs
			     overflow *out* of it, so the ResizeObserver below never sees a tab open
			     or a label change and the scroll bar goes stale. -->
			<div class="flex items-center h-full w-max" role="tablist">
				{#each pinnedLeft as tab (tab.id)}
					{@render tabButton(tab)}
				{/each}

				<div
					class="flex items-center"
					use:dndzone={{
						items: dndMiddle,
						flipDurationMs: 150,
						type: dndType,
						dropTargetStyle: {}
					}}
					onconsider={handleConsider}
					onfinalize={handleFinalize}
				>
					{#each dndMiddle as tab (tab.id)}
						<!-- `flex`, not the default block: an inline-flex tab in a block wrapper
					     sits on a text baseline and rides ~1.5px off the row's centre. -->
						<div class="flex">
							{@render tabButton(tab)}
						</div>
					{/each}
				</div>

				{#each pinnedRight as tab (tab.id)}
					{@render tabButton(tab)}
				{/each}

				{#if afterTabs}
					{@render afterTabs()}
				{/if}
			</div>
		</div>

		{#if overflowing}
			<!-- Decorative: it mirrors the scroll position and can be dragged, but the
			     strip is scrollable without it (wheel, trackpad, and the arrow keys that
			     scroll the focused tab into view), so it stays out of the a11y tree
			     rather than posing as a control at a 4px hit target. -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				aria-hidden="true"
				class="absolute bottom-0 left-0 h-1 rounded-full touch-none bg-hint/0 group-hover/scroll:bg-hint/40 hover:!bg-secondary/60 transition-colors"
				style="width: {thumbWidth}px; transform: translateX({thumbLeft}px);"
				onpointerdown={handleThumbPointerDown}
			></div>
		{/if}
	</div>

	{#if trailing}
		<div class="ml-1 pr-1 flex items-center shrink-0">
			{@render trailing()}
		</div>
	{/if}
</div>
