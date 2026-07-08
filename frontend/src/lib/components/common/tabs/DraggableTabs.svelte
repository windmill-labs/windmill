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
	import ScrollableX from '../ScrollableX.svelte'

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

	// Local list the dnd zone owns. `consider` updates only this (mid-drag it
	// holds svelte-dnd-action's shadow placeholder); we commit to the parent on
	// `finalize` so the placeholder never leaks into a sibling bar.
	let dndMiddle = $state<TabItem[]>(untrack(() => middle))
	let isDragging = false
	$effect(() => {
		const next = middle
		// Re-sync from props except mid-drag, where the dnd zone owns the list.
		if (!isDragging) dndMiddle = next
	})

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
				? 'bg-surface-tertiary text-emphasis'
				: 'bg-transparent text-hint hover:text-secondary'
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

<div class={twMerge('flex items-center bg-surface', c)}>
	<!-- 4px bar to match the strip's `pb-1` reserve. -->
	<ScrollableX class="flex-1 min-w-0 pt-1 pl-1 pb-1" style="--wm-scrollx-size: 4px;">
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
	</ScrollableX>

	{#if trailing}
		<div class="ml-1 pr-1 flex items-center shrink-0">
			{@render trailing()}
		</div>
	{/if}
</div>
