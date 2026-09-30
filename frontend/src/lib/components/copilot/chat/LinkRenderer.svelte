<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ExternalLink, PanelRight } from 'lucide-svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import {
		hasToolDisplayActionHandler,
		runToolDisplayAction
	} from './createdResourceActions.svelte'
	import {
		workspaceItemAction,
		type WindmillItemKind,
		type WorkspaceItemTargetKind
	} from './workspaceItems.svelte'
	import { safeHref } from '$lib/utils/safeHref'

	type Props = {
		href?: string
		children?: Snippet
		'data-wm-kind'?: WindmillItemKind
		'data-wm-path'?: string
		'data-wm-target-kind'?: WorkspaceItemTargetKind
		'data-wm-raw-app'?: string
		title?: string
	}
	let {
		href,
		children,
		'data-wm-kind': wmKind,
		'data-wm-path': wmPath,
		'data-wm-target-kind': wmTargetKind,
		'data-wm-raw-app': wmRawApp,
		title
	}: Props = $props()

	// The drawers ride with the docked chat and the session tabs with the sessions page, so a
	// surface can render this pill with nothing able to open one.
	const available = $derived.by(() => {
		const action = workspaceItemAction(wmKind, wmPath, wmTargetKind, wmRawApp === 'true')
		return action && hasToolDisplayActionHandler(action.type) ? action : undefined
	})
	// Only the preview panel takes the plain click. A drawer stays in the hover menu beside the
	// outbound link: the docked chat mounts drawer handlers on nearly every page, so claiming
	// that click would redirect these pills far outside the sessions page.
	const previewAction = $derived(available?.type === 'open_item_preview' ? available : undefined)

	const allowedHref = $derived(safeHref(href, window.location.href))

	async function openAvailable() {
		if (available) await runToolDisplayAction(available)
	}

	async function onclick(event: MouseEvent) {
		// Modifier clicks still reach the tab from the pill itself, so leave them to the browser.
		if (!previewAction || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return
		event.preventDefault()
		await runToolDisplayAction(previewAction)
	}

	const menuItemClass =
		'flex items-center gap-2 w-full px-2 py-1 rounded-sm text-xs text-primary font-normal no-underline hover:bg-surface-hover transition-colors'
</script>

{#snippet pill()}
	<a
		href={allowedHref}
		target={previewAction ? undefined : '_blank'}
		rel={previewAction ? undefined : 'noopener noreferrer'}
		title={available ? undefined : title || `Open ${wmPath} in a new tab`}
		{onclick}
		class="inline-flex items-baseline gap-1 px-1 rounded hover:bg-surface-hover text-primary no-underline font-mono align-baseline"
	>
		<span class="inline-flex self-center shrink-0">
			<RowIcon kind={wmKind!} size={12} />
		</span>
		{@render children?.()}
	</a>
{/snippet}

{#if allowedHref}
	{#if wmKind && available}
		<!-- The menu floats over the text, so opening it never reflows the sentence. -->
		<Tooltip placement="top-start" openDelay={200} closeDelay={150} customBgClass="bg-surface p-1">
			{@render pill()}
			{#snippet text()}
				<div class="flex flex-col min-w-36">
					<button type="button" class={menuItemClass} onclick={openAvailable}>
						<PanelRight size={14} class="shrink-0" />
						{previewAction ? 'Open in panel' : 'Open in editor'}
					</button>
					<a href={allowedHref} target="_blank" rel="noopener noreferrer" class={menuItemClass}>
						<ExternalLink size={14} class="shrink-0" />
						Open in new tab
					</a>
				</div>
			{/snippet}
		</Tooltip>
	{:else if wmKind}
		{@render pill()}
	{:else}
		<a href={allowedHref} target="_blank" rel="noopener noreferrer" {title}>
			{@render children?.()}
		</a>
	{/if}
{:else}
	<!-- An empty or unsafe href still has text; drop only the link. -->
	{@render children?.()}
{/if}
