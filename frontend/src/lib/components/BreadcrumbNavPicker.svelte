<!--
@component
The chevron after the breadcrumb's last segment: it opens the same picker a session's preview tab
uses — the workspace pages, then the folders with their flows, scripts and apps — and picking a
row navigates the app there.

It lands among the current page's siblings: an item opens in its folder with the item
highlighted, a workspace page inside Pages with that page highlighted.
-->
<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import { goto } from '$app/navigation'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PreviewRouterPicker, { type Scope } from './sessions/PreviewRouterPicker.svelte'
	import { pageKey, type PreviewTarget } from './sessions/previewRouter'
	import { editPathFor, leafKeyFor, viewPathFor } from './workspacePicker'
	import { base } from '$lib/base'
	import type { PageHeaderItem } from './pageHeaderRegistry.svelte'

	let {
		item,
		pagePath,
		actingWorkspaceId,
		iconSize = 14
	}: {
		/** The item the page is about, to land in its folder. */
		item?: PageHeaderItem
		/** The route of a workspace page, to highlight it among the pages. */
		pagePath?: string
		/** Load from, and navigate within, this workspace rather than the one the app points at. */
		actingWorkspaceId?: string
		iconSize?: number
	} = $props()

	let isOpen = $state(false)

	// The item's own folder is its path minus the name.
	const initialScope = $derived.by<Scope>(() => {
		const parts = item?.path?.split('/') ?? []
		return parts.length >= 3 ? { kind: 'all', dir: parts.slice(0, -1).join('/') } : undefined
	})
	const initialHighlight = $derived(
		item?.kind && item.path
			? leafKeyFor(item.kind, item.path)
			: pagePath
				? pageKey(pagePath)
				: undefined
	)

	// A page acting on another workspace than the app's (a session in a fork) keeps acting on it:
	// the route reads `?workspace=` before the store.
	function withWorkspace(href: string): string {
		if (!actingWorkspaceId) return href
		const sep = href.includes('?') ? '&' : '?'
		return `${href}${sep}workspace=${encodeURIComponent(actingWorkspaceId)}`
	}

	function hrefFor(target: PreviewTarget): string | undefined {
		if (target.type === 'page') return target.href
		if (target.type === 'item') {
			const path = target.mode === 'view' ? viewPathFor(target.item) : editPathFor(target.item)
			return `${base}${path}`
		}
		return undefined
	}

	function pick(target: PreviewTarget) {
		isOpen = false
		const href = hrefFor(target)
		if (href) goto(withWorkspace(href))
	}
</script>

<Popover
	placement="bottom-start"
	usePointerDownOutside
	excludeSelectors=".drawer"
	disableFocusTrap
	closeOnOtherPopoverOpen
	bind:isOpen
	openFocus="[data-workspace-picker-search]"
	contentClasses="flex flex-col overflow-hidden"
	class="shrink-0 flex items-center p-1.5 rounded text-tertiary hover:bg-surface-hover hover:text-primary transition-colors"
	triggerAttrs={{ 'aria-label': 'Navigate to', title: 'Navigate to' }}
>
	{#snippet trigger()}
		<ChevronDown size={iconSize} class="flex-shrink-0" />
	{/snippet}
	{#snippet content()}
		<PreviewRouterPicker
			{initialScope}
			{initialHighlight}
			workspaceId={actingWorkspaceId}
			onPick={pick}
		/>
	{/snippet}
</Popover>
