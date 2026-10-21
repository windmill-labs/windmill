<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ON_HOVER, starClass, type Entry, type SectionProps } from '../model'
	import { ago, editedTooltip, keyOf, titleOf } from '../format'
	import KindIcon from '../KindIcon.svelte'
	import ItemMenu from '../ItemMenu.svelte'
	import StarToggle from '../StarToggle.svelte'

	// HomeQuickAccess as it ships, markup and classes copied, so the other variants are
	// compared against the real thing rather than a sketch of it.
	let { entries, config, now, onToggleStar, onForget }: SectionProps = $props()

	const COLLAPSED_COUNT = 9

	let expanded = $state(false)
	let shown = $derived(expanded ? entries : entries.slice(0, COLLAPSED_COUNT))
</script>

{#snippet card(item: Entry)}
	<div
		class="group flex items-center gap-1 rounded-md border border-light bg-surface-tertiary hover:bg-surface-hover transition-colors min-w-0"
	>
		<a
			href="#{item.path}"
			class="flex items-center gap-2 min-w-0 pl-2.5 py-2"
			title={item.summary ? `${item.summary}\n${item.path}` : item.path}
		>
			<span class="shrink-0"><KindIcon kind={item.kind} muted={config.icons === 'muted'} /></span>
			<span class="text-xs font-medium text-emphasis truncate">{titleOf(item)}</span>
		</a>
		<StarToggle
			starred={item.favorite}
			onToggle={() => onToggleStar(item)}
			class={starClass(config)}
		/>
		<a href="#{item.path}" class="flex-1 self-stretch" tabindex="-1" aria-hidden="true"></a>
		{#if !item.favorite && item.at}
			<span
				class="shrink-0 text-2xs text-secondary {config.time === 'hover' ? ON_HOVER : ''}"
				title={editedTooltip(item.at, now)}
				role="note"
			>
				{ago(item.at, now)}
			</span>
		{/if}
		<div class="shrink-0 pr-1 text-secondary {config.menu === 'hover' ? ON_HOVER : ''}">
			<ItemMenu entry={item} {onToggleStar} {onForget} />
		</div>
	</div>
{/snippet}

<section class="flex flex-col gap-2">
	{#if config.label}
		<h2 class="text-xs font-normal text-secondary -mb-1">Favorites & recent</h2>
	{/if}
	<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
		{#each shown as item (keyOf(item))}
			{@render card(item)}
		{/each}
	</div>
	{#if entries.length > COLLAPSED_COUNT}
		<div class="self-start">
			<Button variant="subtle" unifiedSize="xs" onClick={() => (expanded = !expanded)}>
				{expanded ? 'Show less' : `Show all ${entries.length}`}
			</Button>
		</div>
	{/if}
</section>
