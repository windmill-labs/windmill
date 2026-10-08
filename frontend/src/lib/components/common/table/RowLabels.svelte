<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import InheritedLabels from '$lib/components/InheritedLabels.svelte'
	import { isHomeTable } from '$lib/components/home/homeTable'

	interface Props {
		labels: string[] | undefined
		inheritedLabels: string[] | undefined
	}

	let { labels, inheritedLabels }: Props = $props()

	const MAX = 3
	const homeTable = isHomeTable()

	let own = $derived(labels ?? [])
	let inherited = $derived((inheritedLabels ?? []).filter((l) => !own.includes(l)))
	let title = $derived(
		[
			...own.map((l) => 'Label: ' + l),
			...inherited.map((l) => 'Label inherited from folder: ' + l)
		].join('\n')
	)
</script>

{#if homeTable}
	{#if own.length || inherited.length}
		<div class="min-w-0 max-w-full truncate text-xs" {title}>
			<span class="text-secondary">{own.join(', ')}</span>{#if own.length && inherited.length}<span
					class="text-secondary">,&nbsp;</span
				>{/if}<span class="text-hint">{inherited.join(', ')}</span>
		</div>
	{/if}
{:else}
	{#if own.length}
		<div class="flex items-center gap-0.5">
			{#each own.slice(0, MAX) as label (label)}
				<Badge color="blue" small class="px-1" title="Label: {label}">{label}</Badge>
			{/each}
			{#if own.length > MAX}
				<Badge
					color="blue"
					small
					class="px-1"
					title={own
						.slice(MAX)
						.map((l) => 'Label: ' + l)
						.join('\n')}>+{own.length - MAX}</Badge
				>
			{/if}
		</div>
	{/if}
	<InheritedLabels labels={inheritedLabels} />
{/if}
