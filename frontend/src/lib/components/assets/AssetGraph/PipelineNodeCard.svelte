<!--
@component
The body every pipeline graph node shares: icon, a kind line ("Script · draft", "Data table"),
a title, and trailing chips. Node components add only what is theirs — the icon, the chips,
and the controls hung around the card.
-->
<script lang="ts">
	import { NODE } from '$lib/components/graph/util'
	import { PIPELINE_NODE_EXTRA_ROW, PIPELINE_NODE_HEIGHT } from './assetGraphLayout'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'

	interface Props {
		icon: Snippet
		kindLabel: string
		title: string
		/** Hover text; the path, when the title is a summary. */
		tooltip?: string
		/** Scripts sit on the main surface, assets and triggers a step down; in the
		 * assets-only view, which has no scripts, assets take the main surface. */
		surface?: 'primary' | 'secondary'
		selected?: boolean
		/** Not deployed yet: dashed edge. */
		draft?: boolean
		/** Feedback states, which recolor the whole card. `error` is a misconfigured
		 * node, styled like a failed flow step. */
		tone?: 'danger' | 'error' | 'success' | 'running'
		/** Makes the whole card a button. */
		onclick?: (e: MouseEvent) => void
		/** Between the icon and the text: a live status the eye should find first. */
		leading?: Snippet
		trailing?: Snippet
		/** Defaults to `NODE.width`. */
		width?: number
		/** A line under the title, in the text column. */
		subtitle?: Snippet
	}

	let {
		icon,
		kindLabel,
		title,
		tooltip,
		surface = 'secondary',
		selected = false,
		draft = false,
		tone,
		onclick,
		leading,
		trailing,
		subtitle,
		width = NODE.width
	}: Props = $props()

	const cardClass = $derived(
		twMerge(
			'flex items-center w-full text-left font-normal rounded-md drop-shadow-sm overflow-hidden border transition-colors',
			surface === 'primary' ? 'bg-surface' : 'bg-surface-secondary',
			'border-gray-400 dark:border-gray-600 hover:border-gray-500 dark:hover:border-gray-500',
			draft && 'border-dashed border-gray-400 dark:border-gray-500',
			// Like a selected flow step: the fill and the edge colour change, the edge's
			// width and dashes stay.
			selected && 'bg-surface-accent-selected border-border-selected hover:border-border-selected',
			tone === 'danger' &&
				'bg-red-50 dark:bg-red-900/30 border-dashed border-red-400 dark:border-red-500 hover:bg-red-100 dark:hover:bg-red-900/40',
			tone === 'error' &&
				(selected
					? 'bg-red-200 dark:bg-red-600 border-red-500 hover:border-red-500'
					: 'bg-red-100 dark:bg-red-700 border-red-300 dark:border-red-500 hover:border-red-400'),
			tone === 'success' &&
				'bg-green-50 dark:bg-green-900/30 border-green-500 dark:border-green-600',
			tone === 'running' &&
				'bg-amber-50 dark:bg-amber-900/30 border-amber-400 dark:border-amber-600 animate-pulse'
		)
	)
	const minHeight = $derived(PIPELINE_NODE_HEIGHT + (subtitle ? PIPELINE_NODE_EXTRA_ROW : 0))
	const toneText = $derived(
		tone === 'danger'
			? 'text-red-700 dark:text-red-400'
			: tone === 'error'
				? 'text-red-700 dark:text-red-200'
			: tone === 'success'
				? 'text-green-700 dark:text-green-400'
				: undefined
	)
</script>

{#snippet body()}
	<span class="shrink-0 ml-3 mr-2.5 flex items-center">{@render icon()}</span>
	{@render leading?.()}
	<span class="flex flex-col min-w-0 flex-1 pr-1 py-1 leading-tight">
		<span class={twMerge('text-2xs truncate text-secondary', toneText)}>{kindLabel}</span>
		<span class={twMerge('text-xs truncate text-emphasis', selected && 'text-accent', toneText)}
			>{title}</span
		>
		{#if subtitle}
			<span class="flex items-center min-w-0 mt-1">{@render subtitle()}</span>
		{/if}
	</span>
	{#if trailing}
		<span class="flex items-center shrink-0 pr-2">{@render trailing()}</span>
	{/if}
{/snippet}

{#if onclick}
	<button
		type="button"
		{onclick}
		class={cardClass}
		style="width: {width}px; min-height: {minHeight}px;"
		title={tooltip}
	>
		{@render body()}
	</button>
{:else}
	<div
		class={cardClass}
		style="width: {width}px; min-height: {minHeight}px;"
		title={tooltip}
	>
		{@render body()}
	</div>
{/if}
