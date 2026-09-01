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
		// Sweeps a highlight across the label while the row is in progress.
		shimmer?: boolean
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
		shimmer = false,
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
	{#snippet labelText(highlight: boolean)}
		<span
			class={twMerge(
				'text-secondary font-medium text-2xs',
				labelClass,
				highlight && 'text-emphasis'
			)}
		>
			{label}
		</span>
	{/snippet}

	{#snippet headerButton()}
		<button
			class={twMerge(
				'min-w-0 py-0.5 my-0.5 rounded-md hover:bg-surface-hover transition-colors inline-flex items-center gap-2 text-left',
				headerClass
			)}
			onclick={onToggle}
			disabled={!toggleable}
			aria-expanded={toggleable ? expanded : undefined}
		>
			{#if shimmer}
				<span class="shimmer inline-flex items-center min-w-0">
					{@render labelText(false)}
					<span class="shimmer-band inline-flex items-center min-w-0" aria-hidden="true">
						{@render labelText(true)}
					</span>
				</span>
			{:else}
				{@render labelText(false)}
			{/if}
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

<style>
	/* An emphasis-coloured copy of the label sits on top of the muted one and is
	   revealed through a travelling band, so the highlight is a colour change
	   rather than an opacity change and the row keeps its own colour underneath. */
	.shimmer {
		position: relative;
	}
	.shimmer-band {
		position: absolute;
		inset: 0;
		pointer-events: none;
		--wm-shimmer-band: linear-gradient(
			100deg,
			rgba(0, 0, 0, 0.2) 40%,
			rgba(0, 0, 0, 1) 50%,
			rgba(0, 0, 0, 0.2) 60%
		);
		-webkit-mask-image: var(--wm-shimmer-band);
		mask-image: var(--wm-shimmer-band);
		-webkit-mask-size: 250% 100%;
		mask-size: 250% 100%;
		-webkit-mask-repeat: no-repeat;
		mask-repeat: no-repeat;
		animation: wm-shimmer-sweep 2.6s linear infinite;
	}
	/* The travel itself takes 1.5s — the rest of the period holds the band
	   off-screen, so sweeps are spaced out instead of running back to back.
	   250% wide is what puts it off-screen at both ends rather than popping at
	   the edges; the row rests at the gradient's floor in between. */
	@keyframes wm-shimmer-sweep {
		0% {
			-webkit-mask-position: 100% 0;
			mask-position: 100% 0;
		}
		58%,
		100% {
			-webkit-mask-position: 0 0;
			mask-position: 0 0;
		}
	}
	/* The sweep is the only thing marking a row as running, so it degrades to a
	   flat wash rather than disappearing — otherwise a running tool row would be
	   indistinguishable from a settled one here. */
	@media (prefers-reduced-motion: reduce) {
		.shimmer-band {
			animation: none;
			-webkit-mask-image: none;
			mask-image: none;
			opacity: 0.35;
		}
	}
</style>
