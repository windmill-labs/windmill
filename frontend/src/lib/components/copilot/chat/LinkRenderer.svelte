<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ExternalLink, PanelRight } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { runToolDisplayAction } from './createdResourceActions.svelte'
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
		title?: string
	}
	let {
		href,
		children,
		'data-wm-kind': wmKind,
		'data-wm-path': wmPath,
		'data-wm-target-kind': wmTargetKind,
		title
	}: Props = $props()

	const drawerAction = $derived(workspaceItemAction(wmKind, wmPath, wmTargetKind))

	async function openDrawer(event?: Event) {
		event?.preventDefault()
		event?.stopPropagation()
		if (drawerAction) {
			await runToolDisplayAction(drawerAction)
		}
	}
</script>

{#if href}
	{#if wmKind}
		<span class="group inline-flex items-baseline">
			<a
				{href}
				target="_blank"
				rel="noopener noreferrer"
				title={title || wmPath || href}
				class="inline-flex items-baseline gap-1 px-1 rounded hover:bg-surface-hover text-primary no-underline font-mono text-[0.9em] align-baseline"
			>
				<span class="inline-flex self-center shrink-0">
					<RowIcon kind={wmKind} size={12} />
				</span>
				{@render children?.()}
				<span
					class="inline-flex self-center shrink-0 text-tertiary opacity-0 group-hover:opacity-100 transition-opacity"
				>
					<ExternalLink size={10} />
				</span>
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
