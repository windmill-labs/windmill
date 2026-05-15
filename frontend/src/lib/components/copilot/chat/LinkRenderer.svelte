<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Code2, LayoutDashboard } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import type { WindmillItemKind } from './workspaceItems.svelte'

	type Props = {
		href?: string
		children?: Snippet
		'data-wm-kind'?: WindmillItemKind
		'data-wm-path'?: string
		title?: string
	}
	let { href, children, 'data-wm-kind': wmKind, title }: Props = $props()

	// Fallback to URL-based detection if the data attribute didn't make it through
	// (hast/rehype can rename custom properties on some pipelines).
	const kind = $derived.by((): WindmillItemKind | undefined => {
		if (wmKind) return wmKind
		if (!href) return undefined
		if (href.startsWith('/scripts/get/')) return 'script'
		if (href.startsWith('/flows/get/')) return 'flow'
		if (href.startsWith('/apps/get/')) return 'app'
		return undefined
	})
</script>

{#if href}
	{#if kind}
		<a
			{href}
			target="_blank"
			rel="noopener noreferrer"
			title={title || href}
			class="inline-flex items-baseline gap-1 px-1 rounded bg-surface-secondary hover:bg-surface-hover border border-gray-200 dark:border-gray-700 text-primary no-underline font-mono text-[0.9em] align-baseline"
		>
			<span class="inline-flex self-center shrink-0 text-secondary">
				{#if kind === 'script'}
					<Code2 size={12} />
				{:else if kind === 'flow'}
					<BarsStaggered size={12} style="" class="!fill-current" />
				{:else if kind === 'app'}
					<LayoutDashboard size={12} />
				{/if}
			</span>
			{@render children?.()}
		</a>
	{:else}
		<a {href} target="_blank" rel="noopener noreferrer" {title}>
			{@render children?.()}
		</a>
	{/if}
{/if}
