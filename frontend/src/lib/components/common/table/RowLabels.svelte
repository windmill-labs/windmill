<script lang="ts">
	import { Tag } from 'lucide-svelte'
	import LabelBadge from '../badge/LabelBadge.svelte'

	let { labels }: { labels: string[] | undefined | null } = $props()

	// A row has little room beside its title; the rest are counted, and named in the tooltip.
	const SHOWN = 1
</script>

{#if labels?.length}
	<div class="flex items-center gap-0.5">
		{#each labels.slice(0, SHOWN) as label}
			<LabelBadge icon={Tag} title="Label: {label}">{label}</LabelBadge>
		{/each}
		{#if labels.length > SHOWN}
			<LabelBadge
				title={labels
					.slice(SHOWN)
					.map((l) => 'Label: ' + l)
					.join('\n')}>+{labels.length - SHOWN}</LabelBadge
			>
		{/if}
	</div>
{/if}
