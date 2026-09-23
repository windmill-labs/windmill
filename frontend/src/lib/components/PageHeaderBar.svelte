<!--
@component
The bar every route wears, above the page content and beside the sidebar: the sidebar toggle, the
workspace picker, then whatever the route registered — an item's breadcrumb and summary, or a
section name — and the route's own buttons at the far end.

The row's height matches the sidebar's own header row, so the two read as one band.
-->
<script lang="ts">
	import { PanelLeft, PanelTop } from 'lucide-svelte'
	import { navDetached } from './sidebar/navDetached.svelte'
	import { navHandleSlot } from './sidebar/navHandlePlacement.svelte'
	import NavBreadcrumb from './NavBreadcrumb.svelte'
	import { pageHeader } from './pageHeaderRegistry.svelte'
	import ContextBridge from './ContextBridge.svelte'

	let {
		navHidden = false,
		onDock,
		onUnpin
	}: {
		navHidden?: boolean
		/** Sends the band back behind its handle, on a page that owns the viewport. */
		onUnpin?: () => void
		/** Docks the sidebar. The layout owns the two other things that must happen with it — the
		 *  flag that suppresses the rail's entry animation, and closing the card. */
		onDock?: () => void
	} = $props()

	const content = $derived(pageHeader.content)
	const actions = $derived(pageHeader.actions)

	const item = $derived(content?.item)
	const section = $derived(content?.section)
</script>

<div data-page-header class="flex items-center gap-1 h-11 pl-2 pr-4 shrink-0 min-w-0 bg-surface">
	{#if onUnpin}
		<!-- First, before the sidebar's handle: this one is about the band the user is looking at,
		     the other about the sidebar beside it. -->
		<button
			class="flex items-center p-1.5 rounded hover:bg-surface-hover"
			aria-label="Unpin header"
			title="Unpin header"
			onclick={() => onUnpin?.()}
		>
			<PanelTop size={16} class="flex-shrink-0 text-hint" />
		</button>
	{/if}

	{#if navDetached.val && !navHidden}
		<!-- The only way back to a hidden sidebar: hovering slides the card in for a look, a click
		     puts it back for good. Docked, the sidebar speaks for itself and nothing leads the bar. -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			data-nav-handle
			onmouseenter={() => navHandleSlot.open()}
			onmouseleave={() => navHandleSlot.scheduleClose()}
		>
			<!-- No tooltip: hovering here already slides the sidebar in, which says what the button
			     does better than a label popping up over it. -->
			<button
				class="flex items-center p-1.5 rounded hover:bg-surface-hover"
				aria-label="Show sidebar"
				onclick={() => onDock?.()}
			>
				<PanelLeft size={16} class="flex-shrink-0 text-hint" />
			</button>
		</div>
	{/if}

	<!-- The breadcrumb yields width grudgingly (shrink-[0.1]): when a page fills the bar with
	     controls, they are what should narrow, not the name of where the user is. It still gives
	     way rather than pushing them off the bar once there is nothing left to take.
	     An embed (`navHidden`) gets the page's controls without the trail that would offer to
	     navigate the host's workspace. -->
	{#if !navHidden}
		<div class="flex min-w-0 shrink-[0.1]">
			<NavBreadcrumb
				{item}
				{section}
				hint={content?.hint}
				actingWorkspaceId={content?.actingWorkspaceId}
			/>
		</div>
	{/if}

	{#if item && !navHidden && (item.summaryContent || item.summary)}
		<!-- A dot rather than a slash: the summary names the same item the path just located, it is
		     not another level of it. -->
		<span class="shrink-0 text-hint/40 text-xs px-0.5" aria-hidden="true">·</span>
		<!-- No kind icon: the page below is the item, and saying "this is a flow" above a flow
		     editor tells the reader what they can already see. -->
		<div class="flex items-center gap-1 min-w-0">
			{#if item.summaryContent}
				{@render item.summaryContent()}
			{:else}
				<span class="min-w-0 truncate text-xs font-medium text-emphasis">{item.summary}</span>
			{/if}
		</div>
	{/if}

	{#if actions.length > 0}
		<!-- min-w-0, not shrink-0: a page whose actions are a filter row (Runs) puts a control in
		     here that can give width back, and it can only do that if this box may shrink. -->
		<div class="ml-auto flex items-center gap-2 min-w-0 pl-4">
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
