<script lang="ts">
	import { ChevronRight } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'
	import type { Snippet } from 'svelte'

	interface Props {
		label: string
		expanded: boolean
		onToggle: () => void
		// A card with nothing to reveal keeps the header inert (no chevron, no
		// hover affordance implying a toggle).
		toggleable?: boolean
		icon?: Snippet
		// Pinned to the right of the header row, outside the toggle button.
		headerRight?: Snippet
		// Always-visible content between the header and the expandable body.
		belowHeader?: Snippet
		children?: Snippet
		class?: string
		headerClass?: string
		labelClass?: string
		contentClass?: string
	}

	let {
		label,
		expanded,
		onToggle,
		toggleable = true,
		icon,
		headerRight,
		belowHeader,
		children,
		class: className,
		headerClass,
		labelClass,
		contentClass
	}: Props = $props()
</script>

<div class={twMerge('font-mono text-xs', className)}>
	{#snippet headerButton()}
		<button
			class={twMerge(
				'min-w-0 py-0.5 my-0.5 rounded-md hover:bg-surface-hover transition-colors inline-flex items-center gap-2 text-left',
				headerClass
			)}
			onclick={onToggle}
			disabled={!toggleable}
		>
			{@render icon?.()}
			<span class={twMerge('text-primary font-medium text-2xs', labelClass)}>
				{label}
			</span>
			{#if toggleable}
				<ChevronRight
					class={twMerge(
						'w-3 h-3 text-secondary transition-transform duration-150 shrink-0',
						expanded ? 'rotate-90' : ''
					)}
				/>
			{/if}
		</button>
	{/snippet}

	{#if headerRight}
		<div class="flex items-center justify-between gap-2">
			{@render headerButton()}
			{@render headerRight()}
		</div>
	{:else}
		{@render headerButton()}
	{/if}

	{@render belowHeader?.()}

	{#if expanded && children}
		<div
			transition:slide={{ duration: 150 }}
			class={twMerge('border border-border-light rounded-md bg-surface p-3', contentClass)}
		>
			{@render children()}
		</div>
	{/if}
</div>
