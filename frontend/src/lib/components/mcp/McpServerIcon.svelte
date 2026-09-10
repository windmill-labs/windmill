<script lang="ts">
	import type { Component } from 'svelte'
	import { Plug } from 'lucide-svelte'
	import { faviconUrl } from '$lib/utils/faviconUrl'

	/**
	 * A connected server's mark, best available first: the icon Windmill ships for
	 * that integration, then the host's favicon, then a generic plug. The favicon
	 * step is what covers the servers Windmill has no icon for, which is most of
	 * them — MCP's own `icons` field is too new for many servers to populate.
	 */
	let { icon, host, size = 16 }: { icon?: Component<any>; host?: string; size?: number } = $props()

	// Keyed by host rather than a boolean so reconnecting a path to a different
	// server retries instead of inheriting the previous one's failure.
	let failedFor = $state<string | undefined>(undefined)

	const px = $derived(`${size}px`)
</script>

{#if icon}
	{@const Icon = icon}
	<Icon width={px} height={px} />
{:else if host && failedFor !== host}
	<img
		src={faviconUrl(host)}
		alt=""
		width={size}
		height={size}
		class="rounded-sm"
		onerror={() => (failedFor = host)}
	/>
{:else}
	<Plug {size} class="text-tertiary" />
{/if}
