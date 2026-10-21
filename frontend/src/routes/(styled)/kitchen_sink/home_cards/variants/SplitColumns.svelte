<script lang="ts">
	import type { Entry, SectionProps } from '../model'
	import { keyOf } from '../format'
	import ItemRow from '../ItemRow.svelte'
	import SectionHeader from '../SectionHeader.svelte'

	let { entries, config, now, onToggleStar, onForget }: SectionProps = $props()

	const COLLAPSED_COUNT = 5

	let favorites = $derived(entries.filter((e) => e.favorite))
	let recent = $derived(entries.filter((e) => !e.favorite))
	let expanded = $state({ favorites: false, recent: false })
	// With one side empty the other takes the width, flowing into the same two columns: a lone
	// half-width list would leave the right of the page blank.
	let alone = $derived(favorites.length === 0 || recent.length === 0)
</script>

{#snippet column(label: string, list: Entry[], open: boolean, toggle: () => void)}
	<div class="flex min-w-0 flex-col {config.spacing === 'airy' ? 'gap-2' : 'gap-1'}">
		<SectionHeader
			{label}
			showLabel={config.label}
			total={list.length}
			collapsed={alone ? COLLAPSED_COUNT + 1 : COLLAPSED_COUNT}
			expanded={open}
			onToggle={toggle}
		/>
		<div class="-mx-2 grid grid-cols-1 gap-y-0.5 {alone ? 'lg:grid-cols-2 lg:gap-x-14' : ''}">
			{#each open ? list : list.slice(0, alone ? COLLAPSED_COUNT + 1 : COLLAPSED_COUNT) as entry (keyOf(entry))}
				<ItemRow {entry} {config} {now} {onToggleStar} {onForget} />
			{/each}
		</div>
	</div>
{/snippet}

<!-- Side by side from `lg` only: at `md` the sidebar still takes 208px, and two columns of what
     is left truncate nearly every title. -->
<section class="grid grid-cols-1 gap-x-16 gap-y-6 {alone ? '' : 'lg:grid-cols-2'}">
	{#if favorites.length}
		{@render column('Favorites', favorites, expanded.favorites, () => {
			expanded.favorites = !expanded.favorites
		})}
	{/if}
	{#if recent.length}
		{@render column('Edited recently', recent, expanded.recent, () => {
			expanded.recent = !expanded.recent
		})}
	{/if}
</section>
