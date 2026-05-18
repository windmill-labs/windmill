<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ExternalLink } from 'lucide-svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import type { WindmillItemKind } from './workspaceItems.svelte'

	type Props = {
		href?: string
		children?: Snippet
		'data-wm-kind'?: WindmillItemKind
		'data-wm-path'?: string
		title?: string
	}
	let { href, children, 'data-wm-kind': wmKind, 'data-wm-path': wmPath, title }: Props = $props()
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
		</span>
	{:else}
		<a {href} target="_blank" rel="noopener noreferrer" {title}>
			{@render children?.()}
		</a>
	{/if}
{/if}
