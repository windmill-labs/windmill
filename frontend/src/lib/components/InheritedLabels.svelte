<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import LabelBadge from './labels/LabelBadge.svelte'

	interface Props {
		labels: string[] | undefined
		/** The workspace the labels belong to, for their colors. */
		workspace: string | undefined
		max?: number
	}

	let { labels, workspace, max = 3 }: Props = $props()
</script>

{#if labels?.length}
	<div class="flex items-center gap-0.5">
		{#each labels.slice(0, max) as label (label)}
			<LabelBadge {label} {workspace} inherited />
		{/each}
		{#if labels.length > max}
			<Badge
				color="gray"
				small
				class="px-1"
				title={labels
					.slice(max)
					.map((l) => 'Label inherited from folder: ' + l)
					.join('\n')}>+{labels.length - max}</Badge
			>
		{/if}
	</div>
{/if}
