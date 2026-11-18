<script lang="ts">
	import type { Snippet } from 'svelte'
	import { clearOverlayHost } from './overlayHost.svelte'

	let { children }: { children?: Snippet } = $props()

	clearOverlayHost()
</script>

{@render children?.()}
