<script lang="ts" module>
	export type TabItem = {
		/** Stable identifier; used as the `[key]` for dnd and the activeId equality check. */
		id: string
		label: string
		/** Optional lucide-svelte (or compatible) component rendered at 12px before the label. */
		icon?: any
		/** Defaults to true. Set false to hide the × close button. */
		closable?: boolean
		/** Pinned tabs are rendered outside the drag zone — 'left' or 'right' of the draggable group. */
		pinned?: 'left' | 'right'
	}
</script>

<script lang="ts">
	import { dndzone, type DndEvent } from '@windmill-labs/svelte-dnd-action'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

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

	let {
		tabs,
		activeId,
		onSelect,
		onClose,
		onReorder,
		class: c = '',
		trailing
	}: Props = $props()

	const pinnedLeft = $derived(tabs.filter((t) => t.pinned === 'left'))
	const middle = $derived(tabs.filter((t) => !t.pinned))
	const pinnedRight = $derived(tabs.filter((t) => t.pinned === 'right'))

	function rebuild(middleNew: TabItem[]) {
		// dnd-action only owns the middle (draggable) slice; reassemble.
		onReorder?.([...pinnedLeft, ...middleNew, ...pinnedRight])
	}

	function handleConsider(e: CustomEvent<DndEvent<TabItem>>) {
		rebuild(e.detail.items)
	}
	function handleFinalize(e: CustomEvent<DndEvent<TabItem>>) {
		rebuild(e.detail.items)
	}

	function tabClasses(isActive: boolean) {
		return twMerge(
			'group inline-flex items-center gap-1.5 px-3 h-8 text-xs select-none cursor-pointer whitespace-nowrap transition-colors focus:outline-none focus-visible:ring-1 focus-visible:ring-border-selected focus-visible:ring-inset',
			// Active tab shares its background with the content area below it
			// — no border, the boundary "disappears" into the surface.
			// Inactive tabs sit on the darker tab-strip bg and have a subtle
			// right separator so they don't blur together.
			isActive
				? 'bg-surface text-primary'
				: 'bg-surface-secondary text-secondary border-r border-border-light hover:bg-surface-hover hover:text-primary'
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
		// Middle-click closes (browser convention).
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
		class={tabClasses(isActive)}
		onclick={() => onSelect(tab.id)}
		onauxclick={(e) => handleAuxClick(e, tab)}
		onkeydown={(e) => handleKeydown(e, tab)}
	>
		{#if Icon}
			<Icon size={12} />
		{/if}
		<span class="truncate max-w-[180px]">{tab.label}</span>
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

<div
	class={twMerge(
		'flex items-stretch overflow-x-auto bg-surface-secondary',
		c
	)}
	role="tablist"
>
	{#each pinnedLeft as tab (tab.id)}
		{@render tabButton(tab)}
	{/each}

	<div
		class="flex items-stretch"
		use:dndzone={{
			items: middle,
			flipDurationMs: 150,
			type: 'draggable-tabs',
			dropTargetStyle: {}
		}}
		onconsider={handleConsider}
		onfinalize={handleFinalize}
	>
		{#each middle as tab (tab.id)}
			<div>
				{@render tabButton(tab)}
			</div>
		{/each}
	</div>

	{#each pinnedRight as tab (tab.id)}
		{@render tabButton(tab)}
	{/each}

	{#if trailing}
		<div class="ml-auto flex items-center">
			{@render trailing()}
		</div>
	{/if}
</div>
