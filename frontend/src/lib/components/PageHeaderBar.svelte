<!--
@component
The bar every route wears, above the page content and beside the sidebar: the sidebar toggle, the
workspace picker, then whatever the route registered — an item's breadcrumb and summary, or a
section name — and the route's own buttons at the far end.

The row's height matches the sidebar's own header row, so the two read as one band.
-->
<script lang="ts">
	import { PanelLeft } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { navDetached } from './sidebar/navDetached.svelte'
	import { navHandleSlot } from './sidebar/navHandlePlacement.svelte'
	import NavBreadcrumb from './NavBreadcrumb.svelte'
	import WorkspaceItemKindIcon from './WorkspaceItemKindIcon.svelte'
	import { pageHeader } from './pageHeaderRegistry.svelte'
	import ContextBridge from './ContextBridge.svelte'

	let { border = true }: { border?: boolean } = $props()

	const content = $derived(pageHeader.content)
	const actions = $derived(pageHeader.actions)

	// A route can drop the band's bottom edge when its own content draws one. The border stays in
	// the box either way — only its colour goes — so the band's contents do not shift a pixel when
	// a navigation swaps one kind of route for the other.
	const showBorder = $derived(content?.barBorder ?? border)

	const item = $derived(content?.item)
	const section = $derived(content?.section)
</script>

<div
	data-page-header
	class="flex items-center gap-1 h-10 px-2 shrink-0 min-w-0 bg-surface border-b {showBorder
		? ''
		: 'border-transparent'}"
>
	{#if navDetached.val}
		<!-- The only way back to a hidden sidebar: hovering slides the card in for a look, a click
		     puts it back for good. Docked, the sidebar speaks for itself and nothing leads the bar. -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div data-nav-handle onmouseenter={() => navHandleSlot.open()}>
			<Tooltip class="flex" placement="bottom" small>
				<button
					class="flex items-center p-1.5 rounded hover:bg-surface-hover"
					aria-label="Show sidebar"
					onclick={() => (navDetached.val = false)}
				>
					<PanelLeft size={16} class="flex-shrink-0 text-hint" />
				</button>
				{#snippet text()}Show sidebar{/snippet}
			</Tooltip>
		</div>
	{/if}

	<NavBreadcrumb {item} {section} actingWorkspaceId={content?.actingWorkspaceId} />

	{#if item && (item.summaryContent || item.summary)}
		<!-- A dot rather than a slash: the summary names the same item the path just located, it is
		     not another level of it. -->
		<span class="shrink-0 text-hint/40 text-xs px-0.5" aria-hidden="true">·</span>
		<!-- The item's kind belongs with the name a human reads, not with its path. -->
		<div class="flex items-center gap-1 min-w-0">
			<WorkspaceItemKindIcon kind={item.kind} />
			{#if item.summaryContent}
				{@render item.summaryContent()}
			{:else}
				<span class="min-w-0 truncate text-xs font-medium text-emphasis">{item.summary}</span>
			{/if}
		</div>
	{/if}

	{#if actions.length > 0}
		<div class="ml-auto flex items-center gap-2 shrink-0">
			{#each actions as entry, i (i)}
				{#key entry.contexts}
					<ContextBridge contexts={entry.contexts}>
						{@render entry.render()}
					</ContextBridge>
				{/key}
			{/each}
		</div>
	{/if}
</div>
