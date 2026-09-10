<script lang="ts">
	import type { Component } from 'svelte'
	import { Plug } from 'lucide-svelte'

	/**
	 * A connected server's mark, best available first: the icon the MCP server itself
	 * published (`icons`, per the spec), then the icon Windmill ships for that
	 * integration, then a generic plug.
	 *
	 * The server's own icon comes first because it is the authoritative one and costs
	 * nothing to show: `pickMcpIconSrc` admits only `data:` sources, so the bytes are
	 * already here and no request leaves the browser to render them.
	 */
	let { src, icon, size = 16 }: { src?: string; icon?: Component<any>; size?: number } = $props()

	// Keyed by src rather than a boolean so a server publishing a different icon
	// retries instead of inheriting the previous one's failure. A malformed data URI
	// lands here and falls through to the icon below it.
	let failedFor = $state<string | undefined>(undefined)

	const px = $derived(`${size}px`)
</script>

{#if src && failedFor !== src}
	<img
		{src}
		alt=""
		width={size}
		height={size}
		class="rounded-sm"
		onerror={() => (failedFor = src)}
	/>
{:else if icon}
	{@const Icon = icon}
	<Icon width={px} height={px} />
{:else}
	<Plug {size} class="text-tertiary" />
{/if}
