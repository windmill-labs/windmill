<script lang="ts">
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Badge from './common/badge/Badge.svelte'

	interface Props {
		children?: import('svelte').Snippet
	}

	let { children = undefined }: Props = $props()
</script>

<Tooltip>
	<Badge verySmall color="blue" class="px-2">EE</Badge>
	{#snippet text()}
		{#if children}
			{@render children()}
		{:else}
			Enterprise Edition only feature
		{/if}
	{/snippet}
</Tooltip>
