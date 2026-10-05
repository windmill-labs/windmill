<!--
@component
The bar every route wears, above the page content and beside the sidebar: the sidebar toggle, the
workspace picker, then whatever the route registered — an item's breadcrumb and summary, or a
section name — and the route's own buttons at the far end.

The row's height matches the sidebar's own header row, so the two read as one band.
-->
<script lang="ts">
	import { PanelLeft } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { navDetached } from './sidebar/navDetached.svelte'
	import { navHandleSlot } from './sidebar/navHandlePlacement.svelte'
	import NavBreadcrumb from './NavBreadcrumb.svelte'
	import { pageHeader, PHONE_BAR } from './pageHeaderRegistry.svelte'
	import ContextBridge from './ContextBridge.svelte'
	import { afterNavigate } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'

	let {
		navHidden = false,
		panelled = false
	}: {
		navHidden?: boolean
		/** The sidebar is a panel over the page rather than a rail beside it — detached, or a
		 *  window too narrow to seat one. Either way this bar carries the switch that opens it. */
		panelled?: boolean
	} = $props()

	const content = $derived(pageHeader.content)
	const actions = $derived(pageHeader.actions)

	const item = $derived(content?.item)
	const section = $derived(content?.section)

	// Below this, the bar is short enough that the workspace's name is the first thing worth
	// giving up: it is the part of the trail the user changes least and the picker beside it still
	// names every workspace.
	const NARROW_BAR = 1000
	const narrow = $derived(pageHeader.barWidth > 0 && pageHeader.barWidth < NARROW_BAR)
	/** A phone's bar: the trail gives way before the page's buttons do (see the markup). */
	const phone = $derived(pageHeader.barWidth > 0 && pageHeader.barWidth < PHONE_BAR)

	// The band's edge is there to say "something is passing under me", so it is drawn only while
	// something is. Scroll events do not bubble, but they do capture, so one listener on the window
	// sees every box under the band — which matters because most pages scroll inside their own
	// (the audit table, a detail page's panes) and never move this layout's own scroller.
	let scrolledUnder = $state(false)
	/** Sub-pixel layout and a border or two, between a box's top and the content box's. */
	const AT_THE_TOP_PX = 4
	function onScrollCapture(e: Event) {
		const el = e.target as HTMLElement | null
		if (!el || el.nodeType !== 1) return
		const box = document.getElementById('content')
		if (!box || !box.contains(el)) return
		// The answer this event would give, before measuring anything. While it matches what the
		// band already draws there is nothing to decide — which is the steady state of a scroll,
		// and the two rects below each force a synchronous layout. This listener captures every
		// scroller in the app (Monaco, the audit table, a run's logs), about once a frame while any
		// of them moves, so the common path has to stay off the layout.
		const next = el.scrollTop > 0
		if (next === scrolledUnder) return
		// Only a box that starts where the content does has anything passing under the band. The
		// audit page's list begins 285px lower, behind its own chart and column headers, and a
		// select's option list floats wherever it opens: neither moves anything under the header.
		const top = el.getBoundingClientRect().top - box.getBoundingClientRect().top
		if (top > AT_THE_TOP_PX) return
		scrolledUnder = next
	}
	// A page left while scrolled would otherwise hand its edge to the next one, which may have
	// nothing to scroll at all.
	afterNavigate(() => (scrolledUnder = false))

	const separator = $derived(pageHeader.content?.separator ?? 'onScroll')
	const showEdge = $derived(separator === 'always' || (separator === 'onScroll' && scrolledUnder))
</script>

<svelte:window onscrollcapture={onScrollCapture} />

<!-- The band owns its bottom edge, so a page never draws a second one under it. A page says which
     rule it wants (see `separator` in the registry); the default shows the edge only while
     something is passing under the bar. An inset shadow rather than a border: a border would make
     the row 45px and every page below it would move a pixel the moment the edge appeared. -->
<div
	data-page-header
	bind:clientWidth={() => pageHeader.barWidth, (w) => pageHeader.setBarWidth(w)}
	class={twMerge(
		'flex items-center gap-1 shrink-0 min-w-0 bg-surface transition-shadow duration-150',
		phone ? 'flex-wrap content-center min-h-11 py-1 pl-1 pr-2' : 'h-11 pl-2 pr-4',
		// The filling surface is what meets the right edge, so the bar keeps no gutter of its own
		// there — and it cannot wrap, since that surface is as tall as the bar.
		content?.actionsFill && 'pr-0 flex-nowrap',
		// The small left padding is there to sit a button against: the sidebar handle, or the
		// workspace disc, both of which carry their own visual margin. Embedded there is neither,
		// and the page's name would start as bare text 8px from the edge.
		navHidden && 'pl-4',
		showEdge &&
			'shadow-[inset_0_-1px_0_0_rgb(var(--color-border-light))] dark:shadow-[inset_0_-1px_0_0_#374151] [html.github-dark_&]:shadow-[inset_0_-1px_0_0_rgb(var(--color-border-light))]'
	)}
>
	{#if (navDetached.val || panelled) && !navHidden}
		<!-- Reveals the hidden sidebar, and only that: hovering slides the card in, clicking holds
		     it there. Attaching it for good belongs to the toggle in the sidebar's own footer, where
		     detaching it happened — a control that hides the thing it sits on cannot also be the
		     way to bring it back. Docked, the sidebar speaks for itself and nothing leads the bar. -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			data-nav-handle
			onmouseenter={() => navHandleSlot.open()}
			onmouseleave={() => navHandleSlot.scheduleClose()}
		>
			<!-- No tooltip: hovering here already slides the sidebar in, which says what the button
			     does better than a label popping up over it. -->
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				startIcon={{ icon: PanelLeft }}
				aria-label="Show sidebar"
				onclick={() => navHandleSlot.open()}
			/>
		</div>
	{/if}

	<!-- The breadcrumb yields width grudgingly (shrink-[0.1]): when a page fills the bar with
	     controls, they are what should narrow, not the name of where the user is. It still gives
	     way rather than pushing them off the bar once there is nothing left to take.
	     On a phone it stops yielding at 10rem, and that floor is what makes the bar wrap: the
	     page's buttons no longer fit beside a trail that wide, so they take the line below
	     instead of squeezing the name down to an ellipsis.
	     An embed (`navHidden`) keeps the page's own name but drops the workspace part around it:
	     the trail would offer to navigate the host's workspace, while the name is what says which
	     page the controls beside it belong to. -->
	<div class={twMerge('flex min-w-0', phone ? 'shrink min-w-[10rem]' : 'shrink-[0.1]')}>
		<!-- Bridged like the actions below: what a page hangs off its own name renders here, out
		     of the tree that named it, and the pen in there asks that tree who the acting user is
		     before it offers to rename anything. -->
		{#key content?.contexts}
			<ContextBridge contexts={content?.contexts}>
				<NavBreadcrumb
					{item}
					{section}
					{narrow}
					nameOnly={navHidden}
					afterName={content?.afterName}
					actingWorkspaceId={content?.actingWorkspaceId}
				/>
			</ContextBridge>
		{/key}
	</div>

	{#if item && (item.summaryContent || item.summary)}
		<!-- No kind icon: the page below is the item, and saying "this is a flow" above a flow
		     editor tells the reader what they can already see. -->
		<div class={twMerge('flex items-center gap-1 min-w-0', phone && 'shrink')}>
			<!-- A dot rather than a slash: the summary names the same item the path just located,
			     it is not another level of it. Inside this box so that a bar wrapping on a phone
			     carries it down with the summary instead of stranding it on the line above. -->
			<span class="shrink-0 text-hint/40 text-xs px-0.5" aria-hidden="true">·</span>
			{#if item.summaryContent}
				{#key content?.contexts}
					<ContextBridge contexts={content?.contexts}>
						{@render item.summaryContent()}
					</ContextBridge>
				{/key}
			{:else}
				<span class="min-w-0 truncate text-xs font-medium text-emphasis">{item.summary}</span>
			{/if}
		</div>
	{/if}

	{#if actions.length > 0}
		<!-- min-w-0, not shrink-0: a page whose actions are a filter row (Runs) puts a control in
		     here that can give width back, and it can only do that if this box may shrink. A phone
		     only tightens the gutter — pages fold their own buttons into a menu at that width, and
		     a box that refused to shrink would make the one flexible field take its max instead. -->
		<!-- `actionsFill`: the actions are a surface rather than a row, so they take the rest of the
		     bar and run to its right edge, and they stretch its full height because that surface
		     sits on the bottom edge. -->
		<div
			class={twMerge(
				'flex',
				// pl-4: the gap between the breadcrumb and the filling surface is bare bar, so it
				// belongs here rather than inside that surface, where it would be filled.
				content?.actionsFill
					? 'flex-1 min-w-0 self-stretch items-stretch pl-4'
					: 'ml-auto items-center',
				!content?.actionsFill && (phone && !content?.actionsFlexible ? 'shrink-0' : 'min-w-0'),
				!content?.actionsFill && (phone ? 'gap-1 pl-1' : 'gap-2 pl-4')
			)}
		>
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
