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
	import { BAR_HEIGHT, pageHeader, PHONE_BAR } from './pageHeaderRegistry.svelte'
	import ContextBridge from './ContextBridge.svelte'
	import { afterNavigate } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'
	import { onDestroy } from 'svelte'
	import type { Attachment } from 'svelte/attachments'
	import { BAR_STAGES, FLOOR, fitBar, type BarFit, type TrailMeasures } from './pageHeaderStages'

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
	const hasSummary = $derived(!!item && !!(item.summaryContent || item.summary))

	/** A phone's bar: a tighter gutter around the same row. */
	const phone = $derived(pageHeader.barWidth > 0 && pageHeader.barWidth < PHONE_BAR)

	// What the bar drops, and when the page's actions fold, follows from the widths of what is in
	// it (see pageHeaderStages): measured here and in NavBreadcrumb, never guessed per page.
	/** `gap-1` between the bar's boxes. */
	const GAP = 4
	/** Actions that shrink or fill the bar have no width of their own to plan around: the trail
	 *  leaves them this much and they take whatever is left. */
	const FLEXIBLE_ACTIONS_RESERVE = 240

	let trailMeasures = $state<TrailMeasures>({ trail: [], family: 0, fork: 0, name: 0 })
	/** The summary held at its floor, with the dot before it. */
	let summaryFloorWidth = $state(0)
	let summaryTextWidth = $state(0)
	/** A page's own summary control, last seen uncapped. */
	let summaryContentWidth = $state(0)
	let handleWidth = $state(0)
	/** The actions box at each fold level it has been seen at, for this page. */
	let actionWidths = $state<(number | undefined)[]>([])
	let actionsEpoch = $state(0)

	const handleShown = $derived((navDetached.val || panelled) && !navHidden)
	// The bar's own padding, in px, as the classes on the row below set it.
	const padLeft = $derived(navHidden ? 16 : phone ? 4 : 8)
	const padRight = $derived(content?.actionsFill ? 0 : phone ? 8 : 16)
	const actionsRigid = $derived(!content?.actionsFlexible && !content?.actionsFill)
	const FULL: BarFit = { stage: 0, fold: 0, caps: {}, scopesGive: false }

	const fit = $derived.by((): BarFit => {
		// Unmeasured counts as wide: the bar measures itself on mount, and starting narrow would fold
		// a page's buttons for one frame and pop them back out the next.
		if (pageHeader.barWidth === 0) return FULL
		return fitBar({
			barWidth: pageHeader.barWidth,
			chrome: padLeft + padRight + (handleShown ? handleWidth + GAP : 0),
			trail: trailMeasures.trail,
			summary: BAR_STAGES.map(() => (hasSummary ? GAP + summaryFloorWidth : 0)),
			actions:
				actions.length === 0 ? [0] : actionsRigid ? actionWidths : [GAP + FLEXIBLE_ACTIONS_RESERVE],
			folds: actions.length > 0 && actionsRigid ? (content?.actionFolds ?? 0) : 0,
			natural: {
				family: trailMeasures.family,
				fork: trailMeasures.fork,
				summary: hasSummary ? (item?.summaryContent ? summaryContentWidth : summaryTextWidth) : 0,
				name: trailMeasures.name
			}
		})
	})
	const stage = $derived(BAR_STAGES[fit.stage])
	onDestroy(pageHeader.provideActionFold(() => fit.fold))

	// Re-attached whenever the fold level changes, so the first reading at a level is taken even
	// when the row comes out exactly as wide as it was at the last one.
	function measureActions(fold: number, _epoch: number): Attachment<HTMLElement> {
		return (node) => {
			const ro = new ResizeObserver(() => (actionWidths[fold] = GAP + node.offsetWidth))
			ro.observe(node)
			return () => ro.disconnect()
		}
	}
	// Until the row has been seen at the level the bar picked, it is laid out but not shown: the bar
	// may find it does not fit and fold it further, and that should not flash.
	const actionsUnmeasured = $derived(actionsRigid && actionWidths[fit.fold] === undefined)

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
	// nothing to scroll at all. Its actions' widths go with it.
	afterNavigate(({ from, to }) => {
		scrolledUnder = false
		if (from?.url.pathname !== to?.url.pathname) {
			actionWidths = []
			actionsEpoch++
		}
	})

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
		'flex flex-col shrink-0 min-w-0 bg-surface transition-shadow duration-150',
		showEdge &&
			'shadow-[inset_0_-1px_0_0_rgb(var(--color-border-light))] dark:shadow-[inset_0_-1px_0_0_#374151] [html.github-dark_&]:shadow-[inset_0_-1px_0_0_rgb(var(--color-border-light))]'
	)}
>
	<div
		class={twMerge(
			'flex items-center gap-1 min-w-0',
			// No flex-wrap. A wrapping row never squeezes anything — flex moves an item to the next line
			// instead of taking it below its content width — so wrapping here would hand the trail a
			// line of its own and leave it whole. What needs a line of its own asks for one through
			// `belowBar`, at a fold the bar picks.
			phone ? `${BAR_HEIGHT} pl-1 pr-2` : `${BAR_HEIGHT} pl-2 pr-4`,
			// The filling surface is what meets the right edge, so the bar keeps no gutter of its own
			// there.
			content?.actionsFill && 'pr-0',
			// The small left padding is there to sit a button against: the sidebar handle, or the
			// workspace disc, both of which carry their own visual margin. Embedded there is neither,
			// and the page's name would start as bare text 8px from the edge.
			navHidden && 'pl-4'
		)}
	>
		{#if (navDetached.val || panelled) && !navHidden}
			<!-- Reveals the hidden sidebar, and only that: the card follows the pointer, holding while it
		     is on the handle, on the card, or in a menu either opened, and sliding back out when it
		     leaves. Where no pointer hovers, a tap opens it and it stays until dismissed. Attaching
		     it for good belongs to the toggle in the sidebar's own footer, where detaching it
		     happened. Docked, the sidebar speaks for itself and nothing leads the bar. -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				data-nav-handle
				bind:clientWidth={handleWidth}
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

		<!-- The breadcrumb's parts are rigid and the bar sizes them (see `fit`): it drops whole parts
	     and caps the names left, so this box only shrinks past what they add up to on a bar too
	     narrow for even the last stage.
	     An embed (`navHidden`) keeps the page's own name but drops the workspace part around it:
	     the trail would offer to navigate the host's workspace, while the name is what says which
	     page the controls beside it belong to. -->
		<div class="flex min-w-0">
			<!-- Bridged like the actions below: what a page hangs off its own name renders here, out of
		     the tree that named it, and the rename editor in there asks that tree who the acting user
		     is before it offers to rename anything. -->
			<!-- Each fragment the band renders comes from another tree and reads that tree's state. The
		     registry already tolerates a registration that cannot answer while its owner's data is
		     being cleared (see `read`); the same window reaches rendering, and an error here would
		     otherwise travel to the root and blank the app rather than the one fragment. A
		     boundary per fragment keeps that to the fragment. -->
			<svelte:boundary
				onerror={(e) => console.error('page header: breadcrumb failed to render', e)}
			>
				{#key content?.contexts}
					<ContextBridge contexts={content?.contexts}>
						<NavBreadcrumb
							{item}
							{section}
							{stage}
							caps={fit.caps}
							scopesGive={fit.scopesGive}
							bind:measures={trailMeasures}
							nameOnly={navHidden}
							afterName={content?.afterName}
							actingWorkspaceId={content?.actingWorkspaceId}
						/>
					</ContextBridge>
				{/key}
			</svelte:boundary>
		</div>

		{#if item && hasSummary}
			<!-- No kind icon: the page below is the item, and saying "this is a flow" above a flow
		     editor tells the reader what they can already see. -->
			<div class="flex items-center gap-1 shrink-0">
				<!-- A dot after the path: the summary names the same item the path just located, it is not
			     another level of it. With the path dropped it follows the workspace, and the slash is
			     the trail's own separator. Fixed width, so either fits the width measured below. -->
				<span class="shrink-0 w-2 text-center text-hint/40 text-xs" aria-hidden="true"
					>{stage.path ? '·' : '/'}</span
				>
				{#if item.summaryContent}
					<!-- Read only while uncapped: capped, the box says how wide the bar let it be, not how
				     wide the page's control wants to be. -->
					<div
						class="flex min-w-0 overflow-hidden"
						style={fit.caps.summary === undefined ? undefined : `max-width: ${fit.caps.summary}px`}
						bind:clientWidth={
							() => summaryContentWidth,
							(w) => fit.caps.summary === undefined && (summaryContentWidth = w)
						}
					>
						<svelte:boundary
							onerror={(e) => console.error('page header: summary failed to render', e)}
						>
							{#key content?.contexts}
								<ContextBridge contexts={content?.contexts}>
									{@render item.summaryContent()}
								</ContextBridge>
							{/key}
						</svelte:boundary>
					</div>
				{:else}
					<span
						class="truncate text-xs font-medium text-emphasis"
						style={fit.caps.summary === undefined ? undefined : `max-width: ${fit.caps.summary}px`}
						>{item.summary}</span
					>
				{/if}
			</div>
		{/if}

		{#if actions.length > 0}
			<!-- A plain row of buttons holds its width (shrink-0): squeezing it only pushes buttons out
		     of the bar, so the bar makes room by dropping breadcrumb parts and then by asking the
		     page to fold (`actionFolds`). Flexible actions (the Runs filter row) are a control that
		     can give width back, which it can only do if this box may shrink. -->
			<!-- `actionsFill`: the actions are a surface rather than a row, so they take the rest of the
		     bar and run to its right edge, and they stretch its full height because that surface
		     sits on the bottom edge. -->
			<div
				{@attach measureActions(fit.fold, actionsEpoch)}
				class={twMerge(
					'flex',
					// pl-4: the gap between the breadcrumb and the filling surface is bare bar, so it
					// belongs here rather than inside that surface, where it would be filled.
					content?.actionsFill
						? 'flex-1 min-w-0 self-stretch items-stretch pl-4'
						: 'ml-auto items-center',
					!content?.actionsFill && (actionsRigid ? 'shrink-0' : 'min-w-0'),
					!content?.actionsFill && (phone ? 'gap-1 pl-1' : 'gap-2 pl-4'),
					actionsUnmeasured && 'invisible'
				)}
			>
				{#each actions as entry, i (i)}
					<svelte:boundary
						onerror={(e) => console.error('page header: actions failed to render', e)}
					>
						{#key entry.contexts}
							<ContextBridge contexts={entry.contexts}>
								{@render entry.render()}
							</ContextBridge>
						{/key}
					</svelte:boundary>
				{/each}
			</div>
		{/if}

		{#if item && hasSummary}
			<!-- The summary at its floor, which is what a stage that keeps it needs; and whole, which is
		     what the bar caps it from. A page's own control stands in at the width last seen. -->
			<div class="absolute -left-[99999px] top-0 invisible" aria-hidden="true" inert>
				<div class="flex items-center gap-1 w-max" bind:clientWidth={summaryFloorWidth}>
					<span class="shrink-0 w-2 text-center text-hint/40 text-xs">·</span>
					{#if item.summaryContent}
						<span style="width: {Math.min(summaryContentWidth, FLOOR.summary)}px"></span>
					{:else}
						<span class="truncate text-xs font-medium" style="max-width: {FLOOR.summary}px"
							>{item.summary}</span
						>
					{/if}
				</div>
				<span
					class="inline-block whitespace-nowrap text-xs font-medium"
					bind:clientWidth={summaryTextWidth}>{item.summary ?? ''}</span
				>
			</div>
		{/if}
	</div>
	{#if content?.belowBar && fit.fold >= (content.belowBarFold ?? 0)}
		<!-- Same gutter as the row above, so what moved down lines up with the breadcrumb's start. -->
		<div
			class={twMerge('flex items-center pb-2', phone ? 'px-2' : 'pl-3 pr-4', navHidden && 'pl-4')}
		>
			<svelte:boundary
				onerror={(e) => console.error('page header: second row failed to render', e)}
			>
				{#key content?.contexts}
					<ContextBridge contexts={content?.contexts}>
						{@render content.belowBar()}
					</ContextBridge>
				{/key}
			</svelte:boundary>
		</div>
	{/if}
</div>
