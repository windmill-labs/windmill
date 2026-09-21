<script lang="ts">
	import type { Component } from 'svelte'
	import McpIcon from '$lib/components/icons/McpIcon.svelte'
	import { twMerge } from 'tailwind-merge'

	/**
	 * A connected server's mark: the icon Windmill ships for that integration, or the
	 * MCP logo for a server it has none for — which still says what kind of thing the
	 * row reaches, where a generic plug did not.
	 */
	let {
		icon,
		size = 16,
		class: className = ''
	}: { icon?: Component<any>; size?: number; class?: string } = $props()

	const px = $derived(`${size}px`)
</script>

<!-- Decorative: the row's own label names the server, and the MCP logo carries a
	 <title> that would otherwise land in the accessible name of the row it marks. -->
<span class={twMerge('inline-flex shrink-0', className)} aria-hidden="true">
	{#if icon}
		{@const Icon = icon}
		<Icon width={px} height={px} />
	{:else}
		<McpIcon width={size} height={size} />
	{/if}
</span>
