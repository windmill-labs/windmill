<script lang="ts">
	import { Button } from '$lib/components/common'

	interface Props {
		label: string
		showLabel: boolean
		total: number
		collapsed: number
		expanded: boolean
		onToggle: () => void
	}

	let { label, showLabel, total, collapsed, expanded, onToggle }: Props = $props()

	let overflows = $derived(total > collapsed)
</script>

<!-- The label and the way to the rest share one line, so "Show all" costs no row of its own. -->
{#if showLabel || overflows}
	<div class="flex h-5 items-center justify-between gap-2">
		{#if showLabel}
			<h2 class="text-xs font-normal text-secondary">{label}</h2>
		{:else}
			<span></span>
		{/if}
		{#if overflows}
			<Button
				variant="subtle"
				unifiedSize="xs"
				btnClasses="!text-2xs !text-secondary"
				onClick={onToggle}
			>
				{expanded ? 'Show less' : `Show all ${total}`}
			</Button>
		{/if}
	</div>
{/if}
