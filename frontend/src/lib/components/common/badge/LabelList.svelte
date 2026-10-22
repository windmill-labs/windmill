<script lang="ts">
	import LabelBadge from './LabelBadge.svelte'
	import type { BadgeColor, BadgeIconProps } from './model'

	interface Props {
		labels: string[] | undefined | null
		icon: BadgeIconProps['icon']
		/** Names where each label comes from in its tooltip, e.g. "Label". */
		titlePrefix: string
		/** Shown as badges; the rest are counted, and named in the count's tooltip. */
		max?: number
		color?: BadgeColor
	}

	let { labels, icon, titlePrefix, max = 3, color = 'gray' }: Props = $props()
</script>

{#if labels?.length}
	<div class="flex items-center gap-0.5">
		{#each labels.slice(0, max) as label (label)}
			<LabelBadge {color} {icon} title="{titlePrefix}: {label}">{label}</LabelBadge>
		{/each}
		{#if labels.length > max}
			<LabelBadge
				{color}
				title={labels
					.slice(max)
					.map((l) => `${titlePrefix}: ${l}`)
					.join('\n')}>+{labels.length - max}</LabelBadge
			>
		{/if}
	</div>
{/if}
