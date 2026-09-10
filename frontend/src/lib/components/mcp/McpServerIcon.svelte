<script lang="ts">
	import type { Component } from 'svelte'
	import { Plug } from 'lucide-svelte'

	/**
	 * A connected server's mark, best available first: the icon the MCP server itself
	 * published (`icons`, per the spec), then the icon Windmill ships for that
	 * integration, then a generic plug.
	 *
	 * The server's own icon comes first because it is the only source that is both
	 * authoritative and free of a third party — the bytes arrive over the MCP
	 * connection the user already made.
	 */
	let { src, icon, size = 16 }: { src?: string; icon?: Component<any>; size?: number } = $props()

	// Keyed by src rather than a boolean so a server publishing a different icon
	// retries instead of inheriting the previous one's failure. An `https:` src is
	// usually blocked by the app's COEP require-corp anyway, which lands here.
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
