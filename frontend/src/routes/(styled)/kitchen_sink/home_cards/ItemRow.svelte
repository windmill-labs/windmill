<script lang="ts">
	import { ON_HOVER, starClass, type Config, type Entry } from './model'
	import { ago, editedTooltip, titleParts } from './format'
	import KindIcon from './KindIcon.svelte'
	import ItemMenu from './ItemMenu.svelte'
	import StarToggle from './StarToggle.svelte'

	interface Props {
		entry: Entry
		config: Config
		now: number
		onToggleStar: (entry: Entry) => void
		onForget: (entry: Entry) => void
	}

	let { entry, config, now, onToggleStar, onForget }: Props = $props()

	let parts = $derived(titleParts(entry))
</script>

<!-- One shortcut with no frame of its own: a row that lifts on hover, like the tree's rows. -->
<div
	class="group flex h-8 min-w-0 items-center gap-1.5 rounded-md px-2 transition-colors hover:bg-surface-hover"
>
	<a
		href="#{entry.path}"
		class="flex min-w-0 items-center gap-2"
		title={entry.summary ? `${entry.summary}\n${entry.path}` : entry.path}
	>
		<KindIcon kind={entry.kind} muted={config.icons === 'muted'} />
		<span class="truncate text-xs text-primary group-hover:text-emphasis">
			{#if parts.lead}<span class="text-hint">{parts.lead}</span>{/if}{parts.name}
		</span>
	</a>
	<StarToggle
		starred={entry.favorite}
		onToggle={() => onToggleStar(entry)}
		size={12}
		class={starClass(config)}
	/>
	<a href="#{entry.path}" class="flex-1 self-stretch" tabindex="-1" aria-hidden="true"></a>
	{#if !entry.favorite && entry.at}
		<span
			class="shrink-0 text-2xs tabular-nums text-hint {config.time === 'hover' ? ON_HOVER : ''}"
			title={editedTooltip(entry.at, now)}
			role="note"
		>
			{ago(entry.at, now)}
		</span>
	{/if}
	<div class="-mr-1 shrink-0 text-secondary {config.menu === 'hover' ? ON_HOVER : ''}">
		<ItemMenu {entry} {onToggleStar} {onForget} />
	</div>
</div>
