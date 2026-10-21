<script lang="ts">
	import { ON_HOVER, starClass, type SectionProps } from '../model'
	import { ago, editedTooltip, keyOf, kindLabel, titleParts } from '../format'
	import KindIcon from '../KindIcon.svelte'
	import ItemMenu from '../ItemMenu.svelte'
	import StarToggle from '../StarToggle.svelte'
	import SectionHeader from '../SectionHeader.svelte'

	let { entries, config, now, onToggleStar, onForget }: SectionProps = $props()

	let width = $state(0)
	// Columns from the section's own width rather than the viewport's, so one row is always
	// exactly one row: the collapsed tiles fill it and nothing wraps under it.
	let cols = $derived(width >= 960 ? 5 : width >= 700 ? 4 : width >= 480 ? 3 : 2)
	let collapsed = $derived(cols <= 2 ? 4 : cols)
	let expanded = $state(false)
	let shown = $derived(expanded ? entries : entries.slice(0, collapsed))
</script>

<section
	class="flex flex-col {config.spacing === 'airy' ? 'gap-2' : 'gap-1'}"
	bind:clientWidth={width}
>
	<SectionHeader
		label="Favorites & recent"
		showLabel={config.label}
		total={entries.length}
		{collapsed}
		{expanded}
		onToggle={() => (expanded = !expanded)}
	/>
	<div class="grid gap-3" style:grid-template-columns="repeat({cols}, minmax(0, 1fr))">
		{#each shown as entry (keyOf(entry))}
			{@const parts = titleParts(entry)}
			<div
				class="group relative flex min-h-[76px] min-w-0 flex-col gap-2 rounded-lg border border-border-light bg-surface-tertiary p-3 transition-colors hover:bg-surface-hover"
			>
				<!-- The whole tile is the link; the star and the menu sit above it. -->
				<a
					href="#{entry.path}"
					class="absolute inset-0 rounded-lg"
					aria-label={entry.summary || entry.path}
					title={entry.summary ? `${entry.summary}\n${entry.path}` : entry.path}
				></a>
				<div class="flex items-start gap-1">
					<span
						class="line-clamp-2 min-w-0 flex-1 break-words text-xs font-medium leading-snug text-emphasis"
					>
						{#if parts.lead}<span class="font-normal text-hint">{parts.lead}</span>{/if}{parts.name}
					</span>
					<StarToggle
						starred={entry.favorite}
						onToggle={() => onToggleStar(entry)}
						size={12}
						class="relative mt-px {starClass(config)}"
					/>
					<div
						class="relative -mr-1.5 -mt-0.5 shrink-0 text-secondary {config.menu === 'hover'
							? ON_HOVER
							: ''}"
					>
						<ItemMenu {entry} {onToggleStar} {onForget} />
					</div>
				</div>
				<span
					class="mt-auto flex items-center gap-1.5 text-2xs text-hint"
					title={entry.at ? editedTooltip(entry.at, now) : undefined}
				>
					<KindIcon kind={entry.kind} size={12} muted={config.icons === 'muted'} />
					{kindLabel(entry)}
					{#if !entry.favorite && entry.at}
						<span class={config.time === 'hover' ? ON_HOVER : ''}>· {ago(entry.at, now)}</span>
					{/if}
				</span>
			</div>
		{/each}
	</div>
</section>
