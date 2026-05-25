<script lang="ts" module>
	export type TabItem = {
		/** Stable identifier; used as the `[key]` for dnd and the activeId equality check. */
		id: string
		label: string
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

	// Per-instance counter so every DraggableTabs gets its own dnd zone `type`.
	// svelte-dnd-action treats zones sharing a `type` as one connected pool:
	// two bars rendering the same tab ids (e.g. the mirrored single-view bars)
	// would let a drag in one bar mark the matching item as a hidden shadow
	// placeholder in the other and never clear it. Unique types keep them
	// independent. We never drag tabs between two bars, so this loses nothing.
	let dndZoneSeq = 0
</script>

<script lang="ts">
	import { dndzone, type DndEvent } from '@windmill-labs/svelte-dnd-action'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createScrollArea, melt } from '@melt-ui/svelte'
	import { untrack } from 'svelte'

	interface Props {
		tabs: TabItem[]
		activeId: string
		onSelect: (id: string) => void
		onClose?: (id: string) => void
		onReorder?: (newOrder: TabItem[]) => void
		/** Extra classes for the outer tab strip. */
		class?: string
		/** Render after the right-pinned tabs (e.g. a "Split with Preview" toggle). */
		trailing?: import('svelte').Snippet
	}

	let { tabs, activeId, onSelect, onClose, onReorder, class: c = '', trailing }: Props = $props()

	const pinnedLeft = $derived(tabs.filter((t) => t.pinned === 'left'))
	const middle = $derived(tabs.filter((t) => !t.pinned))
	const pinnedRight = $derived(tabs.filter((t) => t.pinned === 'right'))

	// Unique dnd zone type for this instance (see note in the module block).
	const dndType = `draggable-tabs-${dndZoneSeq++}`

	// Local copy of the draggable items that the dnd zone owns and renders.
	// We deliberately do NOT push `handleConsider` updates back to the parent:
	// mid-drag, svelte-dnd-action injects a hidden "shadow placeholder" item
	// into `e.detail.items`. If that reached the parent `tabs`, any sibling
	// DraggableTabs bound to the same `tabs` (e.g. the mirrored single-view
	// bars) would render the placeholder as a `visibility:hidden` ghost and
	// keep it after the drag — the disappearing-tab bug. So consider only
	// updates this local list (live feedback for the dragged bar); we commit
	// to the parent on finalize, when the placeholder is already gone.
	let dndMiddle = $state<TabItem[]>(untrack(() => middle))
	let isDragging = false
	$effect(() => {
		const next = middle
		// Re-sync from props except mid-drag, where the dnd zone owns the list.
		if (!isDragging) dndMiddle = next
	})

	// Melt scroll-area: type='always' keeps the custom thumb's gutter present
	// at all times, so the layout never shifts when content stops/starts
	// overflowing. The native horizontal scrollbar is hidden by melt and our
	// own 4px-tall track sits along the bottom of the strip.
	const {
		elements: { root, viewport, content, scrollbarX, thumbX }
	} = createScrollArea({ type: 'always', dir: 'ltr' })

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
			'group inline-flex items-center gap-1.5 px-2.5 h-7 text-xs rounded-md select-none cursor-pointer whitespace-nowrap transition-colors focus:outline-none focus-visible:ring-1 focus-visible:ring-border-selected focus-visible:ring-inset',
			isActive
				? 'bg-surface-input text-primary'
				: 'bg-transparent text-secondary hover:bg-surface-hover hover:text-primary'
		)
	}

	function handleKeydown(e: KeyboardEvent, tab: TabItem) {
		if (e.key === 'Delete' || e.key === 'Backspace') {
			if (tab.closable !== false) {
				e.preventDefault()
				onClose?.(tab.id)
			}
		} else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
			const idx = tabs.findIndex((t) => t.id === tab.id)
			const next = e.key === 'ArrowLeft' ? idx - 1 : idx + 1
			if (next >= 0 && next < tabs.length) {
				onSelect(tabs[next].id)
				e.preventDefault()
			}
		} else if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			onSelect(tab.id)
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
		aria-selected={isActive}
		tabindex={isActive ? 0 : -1}
		class={twMerge(tabClasses(isActive), tab.closable !== false && 'pr-1')}
		onclick={() => onSelect(tab.id)}
		onauxclick={(e) => handleAuxClick(e, tab)}
		onkeydown={(e) => handleKeydown(e, tab)}
	>
		{#if Icon}
			<Icon size={12} class={tab.iconClass} />
		{/if}
		<span class={twMerge('truncate max-w-[180px]', tab.labelClass)}>{tab.label}</span>
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

<div class={twMerge('flex items-center bg-surface-secondary', c)}>
	<div use:melt={$root} class="tabs-root flex-1 min-w-0 relative pt-1 pl-1 pb-1">
		<div use:melt={$viewport} class="tabs-viewport w-full">
			<!-- Inner flex wrapper — melt's content element is forced to
				 `display: table` which would stack the tabs vertically. -->
			<div use:melt={$content}>
				<div class="flex items-center" role="tablist">
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
							<div>
								{@render tabButton(tab)}
							</div>
						{/each}
					</div>

					{#each pinnedRight as tab (tab.id)}
						{@render tabButton(tab)}
					{/each}
				</div>
			</div>
		</div>

		<div use:melt={$scrollbarX} class="tabs-scrollbar">
			<div use:melt={$thumbX} class="tabs-thumb"></div>
		</div>
	</div>

	{#if trailing}
		<div class="ml-1 pr-1 flex items-center shrink-0">
			{@render trailing()}
		</div>
	{/if}
</div>

<style>
	/* The melt viewport hides its own native scrollbar internally; we just
	   need to give it a height so the content sits flush above the custom
	   bar that lives at the bottom of the root. */
	.tabs-viewport {
		height: 100%;
	}

	/* 4px-tall custom horizontal scrollbar pinned to the bottom of the
	   strip. With melt's `type: 'always'` the track is permanently present,
	   so there's no layout shift when content stops overflowing, and the
	   appearance is identical on macOS, Linux, and Windows regardless of
	   the OS scrollbar setting. */
	:global([data-melt-scroll-area-scrollbar].tabs-scrollbar) {
		height: 4px;
		background: transparent;
		touch-action: none;
		user-select: none;
	}
	:global([data-melt-scroll-area-thumb].tabs-thumb) {
		height: 100%;
		width: var(--melt-scroll-area-thumb-width);
		background: rgb(var(--color-text-hint));
		border-radius: 2px;
		position: relative;
		transition: background-color 0.15s;
	}
	/* When the content fits the viewport melt sets data-state="hidden" but
	   leaves the element at full width — without this rule it looks like a
	   bar spanning the whole track. Hide it so the strip's bottom row is
	   empty until there's actually something to scroll. */
	:global([data-melt-scroll-area-thumb].tabs-thumb[data-state='hidden']) {
		opacity: 0;
	}
	:global([data-melt-scroll-area-thumb].tabs-thumb:hover) {
		background: rgb(var(--color-text-secondary));
	}
</style>
