<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ExternalLink, PanelRight } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import {
		hasToolDisplayActionHandler,
		runToolDisplayAction
	} from './createdResourceActions.svelte'
	import {
		workspaceItemAction,
		type WindmillItemKind,
		type WorkspaceItemTargetKind
	} from './workspaceItems.svelte'

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

	// The drawers ride with the docked chat, so a surface can render this pill with nothing
	// able to open one.
	const available = $derived.by(() => {
		const action = workspaceItemAction(wmKind, wmPath, wmTargetKind, wmRawApp === 'true')
		return action && hasToolDisplayActionHandler(action.type) ? action : undefined
	})
	// Only the preview panel takes the plain click. A drawer keeps its own button beside an
	// outbound link: the docked chat mounts drawer handlers on nearly every page, so claiming
	// that click would redirect these pills far outside the sessions page.
	const previewAction = $derived(available?.type === 'open_item_preview' ? available : undefined)
	const drawerAction = $derived(available?.type === 'open_created_resource' ? available : undefined)

	const hint = $derived(
		previewAction ? `Open ${wmPath} in the preview panel` : `Open ${wmPath} in a new tab`
	)

	async function openDrawer(event?: Event) {
		event?.preventDefault()
		event?.stopPropagation()
		if (drawerAction) {
			await runToolDisplayAction(drawerAction)
		}
	}

	async function onclick(event: MouseEvent) {
		// Modifier clicks are the only remaining route to the tab once the plain click is
		// spoken for, so leave them to the browser.
		if (!previewAction || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return
		event.preventDefault()
		await runToolDisplayAction(previewAction)
	}
</script>

{#if href}
	{#if wmKind}
		<span class="group inline-flex items-baseline">
			<a
				{href}
				target={previewAction ? undefined : '_blank'}
				rel={previewAction ? undefined : 'noopener noreferrer'}
				title={title || hint}
				{onclick}
				class="inline-flex items-baseline gap-1 px-1 rounded hover:bg-surface-hover text-primary no-underline font-mono text-[0.9em] align-baseline"
			>
				<!-- Kind icon and action icon share one fixed 12px box, so the pill is the same
				     width at rest and on hover and the surrounding sentence never reflows. -->
				<span class="relative inline-flex self-center shrink-0 w-3 h-3">
					<span class="absolute inset-0 transition-opacity group-hover:opacity-0">
						<RowIcon kind={wmKind} size={12} />
					</span>
					<span
						class="absolute inset-0 flex items-center justify-center text-tertiary opacity-0 transition-opacity group-hover:opacity-100"
					>
						{#if previewAction}
							<PanelRight size={12} />
						{:else}
							<ExternalLink size={11} />
						{/if}
					</span>
				</span>
				{@render children?.()}
			</a>
			{#if drawerAction}
				<Button
					type="button"
					size="xs3"
					variant="subtle"
					iconOnly
					startIcon={{ icon: PanelRight }}
					title="Open in drawer"
					aria-label="Open {wmPath} in drawer"
					wrapperClasses="ml-0.5 inline-flex self-center shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
					btnClasses="!w-auto !rounded !p-0.5 !text-tertiary"
					onClick={openDrawer}
				/>
			{/if}
		</span>
	{:else}
		<a {href} target="_blank" rel="noopener noreferrer" {title}>
			{@render children?.()}
		</a>
	{/if}
{/if}
