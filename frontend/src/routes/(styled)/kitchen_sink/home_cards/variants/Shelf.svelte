<script lang="ts">
	import { Star } from 'lucide-svelte'
	import type { SectionProps } from '../model'
	import { editedTooltip, keyOf, titleParts } from '../format'
	import KindIcon from '../KindIcon.svelte'
	import SectionHeader from '../SectionHeader.svelte'

	let { entries, config, now }: SectionProps = $props()

	const COLLAPSED_COUNT = 10

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
	<!-- No time and no menu on a pill: the time is in its tooltip, the actions on the item's
	     row in the tree. -->
	<div class="flex flex-wrap gap-1.5">
		{#each shown as entry (keyOf(entry))}
			{@const parts = titleParts(entry)}
			<a
				href="#{entry.path}"
				class="group inline-flex h-7 max-w-[16rem] items-center gap-1.5 rounded-md bg-surface-secondary px-2 text-xs text-primary transition-colors hover:bg-surface-sunken hover:text-emphasis"
				title={[
					entry.summary || entry.path,
					entry.summary ? entry.path : '',
					entry.at ? editedTooltip(entry.at, now) : 'Favorite'
				]
					.filter(Boolean)
					.join('\n')}
			>
				<KindIcon kind={entry.kind} size={13} muted={config.icons === 'muted'} />
				<span class="truncate">
					{#if parts.lead}<span class="text-hint">{parts.lead}</span>{/if}{parts.name}
				</span>
				{#if entry.favorite && config.stars === 'always'}
					<Star size={10} fill="currentcolor" class="shrink-0 text-yellow-500" />
				{/if}
			</a>
		{/each}
	</div>
</section>
