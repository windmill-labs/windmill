<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ExternalLink, PanelRight } from 'lucide-svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import { hasInlineDrawer, type WindmillItemKind } from './workspaceItems.svelte'

	type Props = {
		href?: string
		children?: Snippet
		'data-wm-kind'?: WindmillItemKind
		'data-wm-path'?: string
		title?: string
	}
	let { href, children, 'data-wm-kind': wmKind, 'data-wm-path': wmPath, title }: Props = $props()

	// Fallback to URL-based detection for scripts/flows/apps. Other kinds rely on the
	// data attribute set by the remark plugin.
	const kind = $derived.by((): WindmillItemKind | undefined => {
		if (wmKind) return wmKind
		if (!href) return undefined
		if (href.startsWith('/scripts/get/')) return 'script'
		if (href.startsWith('/flows/get/')) return 'flow'
		if (href.startsWith('/apps/get/')) return 'app'
		return undefined
	})
	const drawerable = $derived(kind ? hasInlineDrawer(kind) : false)
</script>

{#if href}
	{#if kind}
		<span class="group inline-flex items-baseline">
			<a
				{href}
				target="_blank"
				rel="noopener noreferrer"
				title={title || wmPath || href}
				class="inline-flex items-baseline gap-1 px-1 rounded hover:bg-surface-hover text-primary no-underline font-mono text-[0.9em] align-baseline"
			>
				<span class="inline-flex self-center shrink-0">
					<RowIcon {kind} size={12} />
				</span>
				{@render children?.()}
				<span
					class="inline-flex self-center shrink-0 text-tertiary opacity-0 group-hover:opacity-100 transition-opacity"
				>
					<ExternalLink size={10} />
				</span>
			</a>
			{#if drawerable && wmPath}
				<button
					type="button"
					onclick={() => aiChatManager.toggleWorkspaceItemDrawer({ kind, path: wmPath })}
					title="Open in drawer"
					aria-label="Open {wmPath} in drawer"
					class="ml-0.5 inline-flex self-center shrink-0 rounded p-0.5 text-tertiary hover:bg-surface-hover opacity-0 group-hover:opacity-100 transition-opacity"
				>
					<PanelRight size={11} />
				</button>
			{/if}
		</span>
	{:else}
		<a {href} target="_blank" rel="noopener noreferrer" {title}>
			{@render children?.()}
		</a>
	{/if}
{/if}
