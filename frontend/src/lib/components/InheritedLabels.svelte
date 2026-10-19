<script lang="ts">
	import LabelBadge from './common/badge/LabelBadge.svelte'
	import type { BadgeColor } from './common/badge/model'
	import { Folder } from 'lucide-svelte'

	interface Props {
		labels: string[] | undefined
		max?: number
		color?: BadgeColor
	}

	let { labels, max = 3, color = 'gray' }: Props = $props()
</script>

{#if labels?.length}
	<div class="flex items-center gap-0.5">
		{#each labels.slice(0, max) as label (label)}
			<LabelBadge {color} icon={Folder} title="Label inherited from folder: {label}">
				{label}
			</LabelBadge>
		{/each}
		{#if labels.length > max}
			<LabelBadge
				{color}
				title={labels
					.slice(max)
					.map((l) => 'Label inherited from folder: ' + l)
					.join('\n')}>+{labels.length - max}</LabelBadge
			>
		{/if}
	</div>
{/if}
