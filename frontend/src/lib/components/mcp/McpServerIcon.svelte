<script lang="ts">
	import type { Component } from 'svelte'
	import McpIcon from '$lib/components/icons/McpIcon.svelte'

	/**
	 * A connected server's mark: the icon Windmill ships for that integration, or the
	 * MCP logo for a server it has none for — which still says what kind of thing the
	 * row reaches, where a generic plug did not.
	 */
	let { icon, size = 16 }: { icon?: Component<any>; size?: number } = $props()

	const px = $derived(`${size}px`)
</script>

{#if icon}
	{@const Icon = icon}
	<Icon width={px} height={px} />
{:else}
	<McpIcon width={size} height={size} />
{/if}
