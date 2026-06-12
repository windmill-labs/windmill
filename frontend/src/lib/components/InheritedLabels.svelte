<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import { Folder } from 'lucide-svelte'

	interface Props {
		labels: string[] | undefined
		max?: number
	}

	let { labels, max = 3 }: Props = $props()
</script>

{#if labels?.length}
	<div class="flex items-center gap-0.5">
		{#each labels.slice(0, max) as label (label)}
			<Badge color="gray" small class="px-1" title="Label inherited from folder: {label}">
				<Folder size={10} class="mr-0.5 shrink-0" />
				{label}
			</Badge>
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
