<script lang="ts">
	import type { SectionProps } from '../model'
	import { keyOf } from '../format'
	import ItemRow from '../ItemRow.svelte'
	import SectionHeader from '../SectionHeader.svelte'

	interface Props extends SectionProps {
		/** `bare`: rows straight on the page. `tray`: rows in one card of their own.
		 * `embedded`: rows inside the items card, which brings its own frame and padding. */
		look: 'bare' | 'tray' | 'embedded'
	}

	let { entries, config, now, onToggleStar, onForget, look }: Props = $props()

	// Two rows of the three-column grid.
	const COLLAPSED_COUNT = 6

	let expanded = $state(false)
	let shown = $derived(expanded ? entries : entries.slice(0, COLLAPSED_COUNT))
</script>

<section class="flex flex-col {config.spacing === 'airy' ? 'gap-2' : 'gap-1'}">
	<SectionHeader
		label="Favorites & recent"
		showLabel={config.label}
		total={entries.length}
		collapsed={COLLAPSED_COUNT}
		{expanded}
		onToggle={() => (expanded = !expanded)}
	/>
	<!-- Unframed rows hang their hover fill into the gutter, so their icons sit on the same left
	     edge as what is above and below them. -->
	<div
		class="grid grid-cols-1 gap-x-3 gap-y-0.5 sm:grid-cols-2 lg:grid-cols-3 {look === 'tray'
			? 'rounded-lg border border-border-light bg-surface-tertiary p-1.5'
			: '-mx-2'}"
	>
		{#each shown as entry (keyOf(entry))}
			<ItemRow {entry} {config} {now} {onToggleStar} {onForget} />
		{/each}
	</div>
</section>
