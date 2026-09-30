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
	import { pageHeader } from './pageHeaderRegistry.svelte'
	import ContextBridge from './ContextBridge.svelte'

	let {
		navHidden = false,
		hideNavHandle = false,
		onUnpin
	}: {
		navHidden?: boolean
		/** Drops the sidebar's handle, for a page whose own floating control reveals the sidebar
		 *  along with this band — two handles for one gesture would be one too many. */
		hideNavHandle?: boolean
		/** Sends the band back behind its handle, on a page that owns the viewport. */
		onUnpin?: () => void
	} = $props()

	const content = $derived(pageHeader.content)
	const actions = $derived(pageHeader.actions)

	const item = $derived(content?.item)
	const section = $derived(content?.section)
</script>

<div data-page-header class="flex items-center gap-1 h-11 pl-2 pr-4 shrink-0 min-w-0 bg-surface">
	{#if navDetached.val && !navHidden && !hideNavHandle}
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
	     An embed (`navHidden`) gets the page's controls without the trail that would offer to
	     navigate the host's workspace. -->
	{#if !navHidden}
		<div class="flex min-w-0 shrink-[0.1]">
			<NavBreadcrumb
				{item}
				{section}
				afterName={content?.afterName}
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
