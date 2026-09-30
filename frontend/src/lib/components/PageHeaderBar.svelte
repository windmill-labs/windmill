<!--
@component
The bar every route wears, above the page content and beside the sidebar: the sidebar toggle, the
workspace picker, then whatever the route registered — an item's breadcrumb and summary, or a
section name — and the route's own buttons at the far end.

The row's height matches the sidebar's own header row, so the two read as one band.
-->
<script lang="ts">
	import { PanelLeft, PanelTopDashed } from 'lucide-svelte'
	import { navDetached } from './sidebar/navDetached.svelte'
	import { navHandleSlot } from './sidebar/navHandlePlacement.svelte'
	import NavBreadcrumb from './NavBreadcrumb.svelte'
	import { pageHeader, PHONE_BAR } from './pageHeaderRegistry.svelte'
	import ContextBridge from './ContextBridge.svelte'
	import { afterNavigate } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'

	let {
		navHidden = false,
		hideNavHandle = false,
		panelled = false,
		onUnpin
	}: {
		navHidden?: boolean
		/** Drops the sidebar's handle, for a page whose own floating control reveals the sidebar
		 *  along with this band — two handles for one gesture would be one too many. */
		hideNavHandle?: boolean
		/** The sidebar is a panel over the page rather than a rail beside it — detached, or a
		 *  window too narrow to seat one. Either way this bar carries the switch that opens it. */
		panelled?: boolean
		/** Sends the band back behind its handle, on a page that owns the viewport. */
		onUnpin?: () => void
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
		// Only a box that starts where the content does has anything passing under the band. The
		// audit page's list begins 285px lower, behind its own chart and column headers, and a
		// select's option list floats wherever it opens: neither moves anything under the header.
		const top = el.getBoundingClientRect().top - box.getBoundingClientRect().top
		if (top > AT_THE_TOP_PX) return
		scrolledUnder = el.scrollTop > 0
	}
	// A page left while scrolled would otherwise hand its edge to the next one, which may have
	// nothing to scroll at all.
	afterNavigate(() => (scrolledUnder = false))
</script>

<svelte:window onscrollcapture={onScrollCapture} />

<!-- The band draws its own bottom edge, and only while a page is scrolled under it: at the top of
     a page there is nothing to divide, and pages that draw their own first line have dropped it —
     two would stack. An inset shadow rather than a border: a border would make the row 45px and
     every page below it would move a pixel the moment the edge appeared. -->
<div
	data-page-header
	bind:clientWidth={() => pageHeader.barWidth, (w) => pageHeader.setBarWidth(w)}
	class={twMerge(
		'flex items-center gap-1 shrink-0 min-w-0 bg-surface transition-shadow duration-150',
		phone ? 'flex-wrap content-center min-h-11 py-1 pl-1 pr-2' : 'h-11 pl-2 pr-4',
		scrolledUnder &&
			'shadow-[inset_0_-1px_0_0_rgb(var(--color-border-light))] dark:shadow-[inset_0_-1px_0_0_#374151] [html.github-dark_&]:shadow-[inset_0_-1px_0_0_rgb(var(--color-border-light))]'
	)}
>
	{#if (navDetached.val || panelled) && !navHidden && !hideNavHandle}
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
			<button
				class="flex items-center p-1.5 rounded hover:bg-surface-hover"
				aria-label="Show sidebar"
				onclick={() => navHandleSlot.open()}
			>
				<PanelLeft size={16} class="flex-shrink-0 text-hint" />
			</button>
		</div>
	{/if}

	{#if onUnpin}
		<!-- After the sidebar's handle: the sidebar is the outer thing, the band sits inside it. -->
		<button
			class="flex items-center p-1.5 rounded hover:bg-surface-hover"
			aria-label="Unpin the header from deployed apps"
			title="Unpin the header from deployed apps"
			onclick={() => onUnpin?.()}
		>
			<!-- Dashed while the band is shown, solid while it is away: the pair the sidebar's own
			     toggle uses, so the two controls read as one idea. -->
			<PanelTopDashed size={16} class="flex-shrink-0 text-hint" />
		</button>
	{/if}

	<!-- The breadcrumb yields width grudgingly (shrink-[0.1]): when a page fills the bar with
	     controls, they are what should narrow, not the name of where the user is. It still gives
	     way rather than pushing them off the bar once there is nothing left to take.
	     On a phone it stops yielding at 10rem, and that floor is what makes the bar wrap: the
	     page's buttons no longer fit beside a trail that wide, so they take the line below
	     instead of squeezing the name down to an ellipsis.
	     An embed (`navHidden`) gets the page's controls without the trail that would offer to
	     navigate the host's workspace. -->
	{#if !navHidden}
		<div class={twMerge('flex min-w-0', phone ? 'shrink min-w-[10rem]' : 'shrink-[0.1]')}>
			<NavBreadcrumb
				{item}
				{section}
				{narrow}
				afterName={content?.afterName}
				actingWorkspaceId={content?.actingWorkspaceId}
			/>
		</div>
	{/if}

	{#if item && !navHidden && (item.summaryContent || item.summary)}
		<!-- No kind icon: the page below is the item, and saying "this is a flow" above a flow
		     editor tells the reader what they can already see. -->
		<div class={twMerge('flex items-center gap-1 min-w-0', phone && 'shrink')}>
			<!-- A dot rather than a slash: the summary names the same item the path just located,
			     it is not another level of it. Inside this box so that a bar wrapping on a phone
			     carries it down with the summary instead of stranding it on the line above. -->
			<span class="shrink-0 text-hint/40 text-xs px-0.5" aria-hidden="true">·</span>
			{#if item.summaryContent}
				{@render item.summaryContent()}
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
		<div
			class={twMerge(
				'ml-auto flex items-center',
				phone && !content?.actionsFlexible ? 'shrink-0' : 'min-w-0',
				phone ? 'gap-1 pl-1' : 'gap-2 pl-4'
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
