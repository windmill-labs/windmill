<script lang="ts">
	import type { Snippet } from 'svelte'
	import Badge from './Badge.svelte'
	import type { BadgeColor, BadgeIconProps } from './model'

	/**
	 * How a label, an owner or a folder reads wherever an item is tagged with it: the row's own
	 * labels and Home's quick filters both draw it, so the two cannot drift apart.
	 */
	interface Props {
		icon?: BadgeIconProps['icon']
		color?: BadgeColor
		title?: string
		/** Makes it a filter: clickable, and marked with × once selected. */
		onclick?: (event: MouseEvent) => void
		selected?: boolean
		children?: Snippet
	}

	let { icon: Icon, color = 'subtle', title, onclick, selected = false, children }: Props = $props()
</script>

<Badge
	{color}
	{title}
	{onclick}
	clickable={!!onclick}
	{selected}
	class="inline-flex items-center gap-1 px-1.5 align-middle"
>
	{#if Icon}
		<Icon size={12} class="shrink-0" />
	{/if}
	{@render children?.()}
	{#if selected}&cross;{/if}
</Badge>
