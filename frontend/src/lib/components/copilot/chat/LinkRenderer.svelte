<script lang="ts">
	import type { Snippet } from 'svelte'

	type Props = {
		href?: string
		children?: Snippet
	}
	let { href, children }: Props = $props()
</script>

{#if href}
	<a {href} target="_blank" rel="noopener noreferrer">
		{@render children?.()}
	</a>
{/if}
